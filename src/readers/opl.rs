use core::str;
use itertools::Itertools;
use rayon::prelude::*;
use std::collections::HashMap;
use std::io::{empty, BufRead};
use std::mem;
use std::sync::mpsc::{channel, Sender};

use crate::{
    chunks::{Chunk, ChunkBuilder},
    elements::{Element, ElementType, Member, Metadata, SimpleElementType},
    readers::Reader,
    threadpools::READER_THREAD_POOL,
};

#[derive(Debug)]
enum OplElementType {
    Node { lat: Option<f64>, lon: Option<f64> },
    Way { nodes: Option<Vec<i64>> },
    Relation { members: Option<Vec<Member>> },
}

impl From<OplElementType> for ElementType {
    fn from(value: OplElementType) -> Self {
        match value {
            OplElementType::Node { lat, lon } => ElementType::Node {
                lat: lat.unwrap(),
                lon: lon.unwrap(),
            },
            OplElementType::Way { nodes } => ElementType::Way {
                nodes: nodes.unwrap(),
            },
            OplElementType::Relation { members } => ElementType::Relation {
                members: members.unwrap(),
            },
        }
    }
}

#[derive(Debug, Default)]
struct OplElement {
    id: Option<i64>,
    version: Option<i32>,
    visible: Option<bool>,
    changeset: Option<i64>,
    timestamp: Option<String>,
    user_id: Option<i32>,
    username: Option<String>,
    tags: Option<HashMap<String, String>>,
    element_type: Option<OplElementType>,
}

impl From<OplElement> for Element {
    fn from(value: OplElement) -> Self {
        let id = value.id.unwrap();
        let tags = value.tags.unwrap();
        let element_type = ElementType::from(value.element_type.unwrap());
        Element {
            id,
            tags,
            element_type,
            changeset: value.changeset,
            visible: value.visible,
            timestamp: value.timestamp,
            uid: value.user_id,
            user: value.username,
            version: value.version,
        }
    }
}

fn unescape_str(input: &str) -> String {
    let mut output = String::new();
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            let mut hex = String::new();
            while let Some(&next_char) = chars.peek() {
                if next_char == '%' {
                    chars.next(); // consume the closing '%'
                    break;
                }
                hex.push(chars.next().unwrap());
            }
            if let Ok(code_point) = u32::from_str_radix(&hex, 16) {
                if let Some(out_char) = std::char::from_u32(code_point) {
                    output.push(out_char);
                }
            }
        } else {
            output.push(c);
        }
    }
    output
}

fn str_or_fail(value: &[u8]) -> &str {
    str::from_utf8(value).expect("Invalid UTF-8 in input file")
}

fn add_byte_field(field: &[u8], opl_element: &mut OplElement) {
    let (flag, value) = field.split_at(1);
    macro_rules! value_as {
        ($type:ty) => {
            str_or_fail(value).parse::<$type>().unwrap()
        };
    }
    match flag {
        b"n" => {
            opl_element.id = Some(value_as!(i64));
        }
        b"w" => {
            opl_element.id = Some(value_as!(i64));
        }
        b"r" => {
            opl_element.id = Some(value_as!(i64));
        }
        b"v" => {
            opl_element.version = Some(value_as!(i32));
        }
        b"d" => match value {
            b"V" => opl_element.visible = Some(true),
            b"D" => opl_element.visible = Some(false),
            _ => {
                panic!("Deleted field value not recognized: {:?}", field);
            }
        },
        b"c" => {
            opl_element.changeset = Some(value_as!(i64));
        }
        b"t" => {
            opl_element.timestamp = Some(str_or_fail(value).to_string());
        }
        b"i" => {
            opl_element.user_id = Some(value_as!(i32));
        }
        b"u" => {
            opl_element.username = Some(unescape_str(str_or_fail(value)));
        }
        b"T" => {
            let tags: HashMap<String, String> = str_or_fail(value)
                .split(',')
                .filter_map(|t| t.split_once('='))
                .map(|(k, v)| (unescape_str(k), unescape_str(v)))
                .collect();
            opl_element.tags = Some(tags);
        }
        b"x" => match opl_element.element_type {
            Some(OplElementType::Node { lat, .. }) => {
                opl_element.element_type = Some(OplElementType::Node {
                    lat,
                    lon: Some(value_as!(f64)),
                });
            }
            None => {
                opl_element.element_type = Some(OplElementType::Node {
                    lat: None,
                    lon: Some(value_as!(f64)),
                });
            }
            _ => {
                panic!("Longitude set for a non-node element!");
            }
        },
        b"y" => match opl_element.element_type {
            Some(OplElementType::Node { lon, .. }) => {
                opl_element.element_type = Some(OplElementType::Node {
                    lat: Some(value_as!(f64)),
                    lon,
                });
            }
            None => {
                opl_element.element_type = Some(OplElementType::Node {
                    lat: Some(value_as!(f64)),
                    lon: None,
                });
            }
            _ => {
                panic!("Latitude set for a non-node element!");
            }
        },
        b"N" => {
            let nodes: Vec<i64> = value
                .split(|&b| b == b',')
                .map(|node_entry| {
                    let parts: Vec<&[u8]> = node_entry.split(|&c| c == b'x' || c == b'y').collect();
                    str_or_fail(&parts[0][1..]).parse::<i64>().unwrap()
                })
                .collect();

            opl_element.element_type = Some(OplElementType::Way { nodes: Some(nodes) });
        }
        b"M" => {
            let members: Vec<Member> = value
                .split(|&b| b == b',')
                .filter_map(|member| {
                    let member = str_or_fail(member);
                    let (ref_part, role) = member.split_once('@').unwrap();
                    let (type_char, member_id) = ref_part.split_at(1);
                    let member_type = match type_char {
                        "n" => SimpleElementType::Node,
                        "w" => SimpleElementType::Way,
                        "r" => SimpleElementType::Relation,
                        _ => return None,
                    };
                    Some(Member {
                        t: Some(member_type),
                        id: member_id.parse().ok().unwrap(),
                        role: Some(unescape_str(role)),
                    })
                })
                .collect();
            opl_element.element_type = Some(OplElementType::Relation {
                members: Some(members),
            });
        }
        _ => {
            panic!("Unrecognized field: {:?}", field);
        }
    }
}

fn convert_chunk(index: usize, chunk: Box<[Vec<u8>]>) -> Chunk {
    let mut elements = Vec::with_capacity(chunk.len());
    for line in chunk.iter() {
        let mut opl_element = OplElement::default();
        let mut field_start = 0;
        for (i, &b) in line.iter().enumerate() {
            if b == b' ' {
                if field_start < i {
                    add_byte_field(&line[field_start..i], &mut opl_element);
                }
                field_start = i + 1;
            }
        }
        if field_start < line.len() {
            add_byte_field(&line[field_start..], &mut opl_element);
        }
        elements.push(Element::from(opl_element));
    }
    Chunk {
        index,
        elements: elements.into_boxed_slice(),
    }
}

pub struct OplReader {
    pub src: Box<dyn BufRead + Send>,
}

impl Reader for OplReader {
    fn read(
        &mut self,
        chunk_builder: ChunkBuilder,
        sender: Sender<Chunk>,
        metadata_sender: Sender<Metadata>,
    ) {
        // create an empty Metadata object
        let metadata = Metadata::default();

        // send metadata to main thread
        metadata_sender
            .send(metadata)
            .expect("Couldn't send metdata to main thread!");

        let src = mem::replace(&mut self.src, Box::new(empty()));

        let (chunk_sender, chunk_receiver) = channel();

        std::thread::spawn({
            move || {
                src.split(b'\n')
                    .map(|s| s.expect("Unable to read input file buffer"))
                    .chunks(chunk_builder.max_size)
                    .into_iter()
                    .for_each(|chunk| {
                        chunk_sender
                            .send(chunk.collect::<Vec<Vec<u8>>>().into_boxed_slice())
                            .expect("Unable to send chunk of vectors to channel");
                    });
            }
        });

        READER_THREAD_POOL.install(|| {
            chunk_receiver
                .into_iter()
                .enumerate()
                .par_bridge()
                .map(|(index, chunk)| convert_chunk(index, chunk))
                .for_each(|c| {
                    sender
                        .send(c)
                        .expect("Unable to send chunk of elements to channel")
                });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unescape_str() {
        assert_eq!(unescape_str("%20%"), String::from(" "));
        assert_eq!(unescape_str("%2c%"), String::from(","));
        assert_eq!(unescape_str("%2c%%2c%"), String::from(",,"));
        assert_eq!(unescape_str("%1f631%"), String::from("😱"));
        assert_eq!(unescape_str("%12108%"), String::from("𒄈"));
    }
}
