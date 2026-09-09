//! Temporary storage for decoded element chunks.
//!
//! When the input cannot be read twice (for example, standard input), the
//! discovery pass of reference-preserving filtering writes every decoded chunk
//! to a spool file, and the emission pass reads the chunks back. Chunks are
//! stored exactly as decoded, before any filter mutation, so that emission
//! evaluates the same inputs discovery did.
//!
//! Discovery workers append chunks concurrently, so the file holds them in an
//! arbitrary order. The reader replays them in ascending index order, which
//! the parallel runner's back-pressure relies on (see
//! [`crate::readers::Reader`]).

use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufWriter, Read, Seek, SeekFrom, Write},
    vec::IntoIter,
};

use crate::{
    SkywayError,
    chunks::ElementChunk,
    elements::{Element, ElementType, Member, SimpleElementType},
};

/// Where one chunk lives in the spool file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SpoolRecord {
    index: usize,
    offset: u64,
    len: usize,
}

/// Appends encoded chunks to an anonymous temporary file.
pub struct SpoolWriter {
    file: BufWriter<File>,
    position: u64,
    records: Vec<SpoolRecord>,
}

impl SpoolWriter {
    pub fn new() -> io::Result<Self> {
        Ok(SpoolWriter {
            file: BufWriter::new(tempfile::tempfile()?),
            position: 0,
            records: Vec::new(),
        })
    }

    /// Append one encoded chunk, remembering its index.
    pub fn append(&mut self, index: usize, bytes: &[u8]) -> io::Result<()> {
        self.file.write_all(bytes)?;
        self.records.push(SpoolRecord {
            index,
            offset: self.position,
            len: bytes.len(),
        });
        self.position += bytes.len() as u64;
        Ok(())
    }

    /// Finish writing, producing a reader that yields chunks in index order.
    ///
    /// Fails with [`io::ErrorKind::InvalidData`] if the indices written are
    /// not exactly `0..n`: a gap or a duplicate would break the runner's
    /// sequence restoration and its back-pressure.
    pub fn finish(self) -> io::Result<SpoolReader> {
        let mut file = self.file.into_inner().map_err(|e| e.into_error())?;
        file.seek(SeekFrom::Start(0))?;

        let mut records = self.records;
        records.sort_by_key(|record| record.index);
        for (expected, record) in records.iter().enumerate() {
            if record.index != expected {
                let what = if record.index < expected {
                    "duplicate"
                } else {
                    "missing"
                };
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("spool file has a {what} chunk index (expected {expected})"),
                ));
            }
        }

        Ok(SpoolReader {
            file,
            position: 0,
            records: records.into_iter(),
        })
    }
}

/// Reads encoded chunks back in ascending index order.
pub struct SpoolReader {
    file: File,
    position: u64,
    records: IntoIter<SpoolRecord>,
}

impl SpoolReader {
    fn read(&mut self, record: SpoolRecord) -> io::Result<Vec<u8>> {
        if self.position != record.offset {
            self.file.seek(SeekFrom::Start(record.offset))?;
            self.position = record.offset;
        }
        let mut bytes = vec![0u8; record.len];
        self.file.read_exact(&mut bytes)?;
        self.position += record.len as u64;
        Ok(bytes)
    }
}

impl Iterator for SpoolReader {
    type Item = io::Result<(usize, Vec<u8>)>;

    fn next(&mut self) -> Option<Self::Item> {
        let record = self.records.next()?;
        Some(self.read(record).map(|bytes| (record.index, bytes)))
    }
}

const TYPE_NODE: u8 = 1;
const TYPE_WAY: u8 = 2;
const TYPE_RELATION: u8 = 3;

struct Encoder {
    bytes: Vec<u8>,
}

impl Encoder {
    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn i32(&mut self, value: i32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn i64(&mut self, value: i64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn str(&mut self, value: &str) {
        self.u32(value.len() as u32);
        self.bytes.extend_from_slice(value.as_bytes());
    }

    fn option<T>(&mut self, value: Option<T>, write: impl FnOnce(&mut Self, T)) {
        match value {
            Some(v) => {
                self.u8(1);
                write(self, v);
            }
            None => self.u8(0),
        }
    }

    fn simple_type(&mut self, value: &SimpleElementType) {
        self.u8(match value {
            SimpleElementType::Node => TYPE_NODE,
            SimpleElementType::Way => TYPE_WAY,
            SimpleElementType::Relation => TYPE_RELATION,
        });
    }

    fn element(&mut self, element: &Element) {
        self.i64(element.id);
        self.option(element.changeset, Self::i64);
        self.option(element.user.as_deref(), Self::str);
        self.option(element.version, Self::i32);
        self.option(element.uid, Self::i32);
        self.option(element.timestamp.as_deref(), Self::str);
        self.option(element.visible, |e, v| e.u8(v as u8));

        self.u32(element.tags.len() as u32);
        for (key, value) in &element.tags {
            self.str(key);
            self.str(value);
        }

        match &element.element_type {
            ElementType::Node { lat, lon } => {
                self.u8(TYPE_NODE);
                self.i32(*lat);
                self.i32(*lon);
            }
            ElementType::Way { nodes } => {
                self.u8(TYPE_WAY);
                self.u32(nodes.len() as u32);
                for node in nodes {
                    self.i64(*node);
                }
            }
            ElementType::Relation { members } => {
                self.u8(TYPE_RELATION);
                self.u32(members.len() as u32);
                for member in members {
                    self.option(member.t.as_ref(), Self::simple_type);
                    self.i64(member.id);
                    self.option(member.role.as_deref(), Self::str);
                }
            }
        }
    }
}

/// Encode the elements of a chunk.
pub fn encode_chunk(elements: &[Element]) -> Vec<u8> {
    let mut encoder = Encoder { bytes: Vec::new() };
    encoder.u32(elements.len() as u32);
    for element in elements {
        encoder.element(element);
    }
    encoder.bytes
}

struct Decoder<'a> {
    bytes: &'a [u8],
    position: usize,
}

fn corrupt(what: &str) -> SkywayError {
    SkywayError::UnexpectedError(format!("corrupt spool file: {what}"))
}

impl<'a> Decoder<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], SkywayError> {
        let end = self
            .position
            .checked_add(n)
            .filter(|end| *end <= self.bytes.len())
            .ok_or_else(|| corrupt("unexpected end of chunk"))?;
        let slice = &self.bytes[self.position..end];
        self.position = end;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8, SkywayError> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, SkywayError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn i32(&mut self) -> Result<i32, SkywayError> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn i64(&mut self) -> Result<i64, SkywayError> {
        Ok(i64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }

    fn str(&mut self) -> Result<String, SkywayError> {
        let length = self.u32()? as usize;
        let bytes = self.take(length)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| corrupt("invalid UTF-8"))
    }

    fn option<T>(
        &mut self,
        read: impl FnOnce(&mut Self) -> Result<T, SkywayError>,
    ) -> Result<Option<T>, SkywayError> {
        match self.u8()? {
            0 => Ok(None),
            1 => Ok(Some(read(self)?)),
            _ => Err(corrupt("invalid option tag")),
        }
    }

    fn simple_type(&mut self) -> Result<SimpleElementType, SkywayError> {
        match self.u8()? {
            TYPE_NODE => Ok(SimpleElementType::Node),
            TYPE_WAY => Ok(SimpleElementType::Way),
            TYPE_RELATION => Ok(SimpleElementType::Relation),
            _ => Err(corrupt("invalid element type")),
        }
    }

    fn element(&mut self) -> Result<Element, SkywayError> {
        let id = self.i64()?;
        let changeset = self.option(Self::i64)?;
        let user = self.option(Self::str)?;
        let version = self.option(Self::i32)?;
        let uid = self.option(Self::i32)?;
        let timestamp = self.option(Self::str)?;
        let visible = self.option(|d| Ok(d.u8()? != 0))?;

        let tag_count = self.u32()? as usize;
        let mut tags = HashMap::with_capacity(tag_count);
        for _ in 0..tag_count {
            let key = self.str()?;
            let value = self.str()?;
            tags.insert(key, value);
        }

        let element_type = match self.u8()? {
            TYPE_NODE => ElementType::Node {
                lat: self.i32()?,
                lon: self.i32()?,
            },
            TYPE_WAY => {
                let count = self.u32()? as usize;
                let mut nodes = Vec::with_capacity(count);
                for _ in 0..count {
                    nodes.push(self.i64()?);
                }
                ElementType::Way { nodes }
            }
            TYPE_RELATION => {
                let count = self.u32()? as usize;
                let mut members = Vec::with_capacity(count);
                for _ in 0..count {
                    let t = self.option(Self::simple_type)?;
                    let id = self.i64()?;
                    let role = self.option(Self::str)?;
                    members.push(Member { t, id, role });
                }
                ElementType::Relation { members }
            }
            _ => return Err(corrupt("invalid element type")),
        };

        Ok(Element {
            changeset,
            user,
            version,
            uid,
            id,
            timestamp,
            visible,
            tags,
            element_type,
        })
    }
}

/// Decode a chunk produced by [`encode_chunk`].
pub fn decode_chunk(index: usize, bytes: &[u8]) -> Result<ElementChunk, SkywayError> {
    let mut decoder = Decoder { bytes, position: 0 };

    let count = decoder.u32()? as usize;
    let mut elements = Vec::with_capacity(count);
    for _ in 0..count {
        elements.push(decoder.element()?);
    }

    if decoder.position != bytes.len() {
        return Err(corrupt("trailing bytes after chunk"));
    }

    Ok(ElementChunk {
        index,
        content: elements.into_boxed_slice(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_elements() -> Vec<Element> {
        vec![
            Element {
                changeset: Some(12),
                user: Some("mapper".to_string()),
                version: Some(3),
                uid: Some(7),
                id: -5,
                timestamp: Some("2020-01-01T00:00:00Z".to_string()),
                visible: Some(true),
                tags: HashMap::from([("name".to_string(), "Ünïcode ☃".to_string())]),
                element_type: ElementType::Node {
                    lat: 375297835,
                    lon: -774510304,
                },
            },
            Element {
                changeset: None,
                user: None,
                version: None,
                uid: None,
                id: 9,
                timestamp: None,
                visible: None,
                tags: HashMap::new(),
                element_type: ElementType::Way {
                    nodes: vec![1, -5, i64::MAX],
                },
            },
            Element {
                changeset: None,
                user: Some(String::new()),
                version: Some(0),
                uid: None,
                id: 2,
                timestamp: None,
                visible: Some(false),
                tags: HashMap::from([
                    ("a".to_string(), "".to_string()),
                    ("".to_string(), "b".to_string()),
                ]),
                element_type: ElementType::Relation {
                    members: vec![
                        Member {
                            t: Some(SimpleElementType::Way),
                            id: 9,
                            role: Some("outer".to_string()),
                        },
                        Member {
                            t: None,
                            id: 1,
                            role: None,
                        },
                    ],
                },
            },
        ]
    }

    #[test]
    fn chunk_round_trips() {
        let elements = sample_elements();
        let bytes = encode_chunk(&elements);
        let decoded = decode_chunk(4, &bytes).unwrap();

        assert_eq!(decoded.index, 4);
        assert_eq!(Vec::from(decoded.content), elements);
    }

    #[test]
    fn empty_chunk_round_trips() {
        let decoded = decode_chunk(0, &encode_chunk(&[])).unwrap();
        assert!(decoded.content.is_empty());
    }

    #[test]
    fn truncated_chunk_is_an_error() {
        let bytes = encode_chunk(&sample_elements());
        assert!(decode_chunk(0, &bytes[..bytes.len() - 1]).is_err());
        assert!(decode_chunk(0, &[]).is_err());
    }

    #[test]
    fn spool_file_round_trips_in_index_order() {
        let mut writer = SpoolWriter::new().unwrap();
        let first = encode_chunk(&sample_elements());
        let second = encode_chunk(&[]);
        let third = encode_chunk(&sample_elements()[..1]);
        writer.append(1, &first).unwrap();
        writer.append(2, &third).unwrap();
        writer.append(0, &second).unwrap();

        let records: Vec<(usize, Vec<u8>)> = writer
            .finish()
            .unwrap()
            .map(|record| record.unwrap())
            .collect();

        assert_eq!(records, vec![(0, second), (1, first), (2, third)]);
    }

    #[test]
    fn empty_spool_file_yields_nothing() {
        let writer = SpoolWriter::new().unwrap();
        assert_eq!(writer.finish().unwrap().count(), 0);
    }

    #[test]
    fn spool_file_with_a_gap_is_an_error() {
        let mut writer = SpoolWriter::new().unwrap();
        writer.append(0, &encode_chunk(&[])).unwrap();
        writer.append(2, &encode_chunk(&[])).unwrap();

        let error = writer.finish().err().unwrap();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn spool_file_with_a_duplicate_is_an_error() {
        let mut writer = SpoolWriter::new().unwrap();
        writer.append(0, &encode_chunk(&[])).unwrap();
        writer.append(1, &encode_chunk(&[])).unwrap();
        writer.append(1, &encode_chunk(&[])).unwrap();

        let error = writer.finish().err().unwrap();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }
}
