use core::str;
use itertools::Itertools;
use rayon::prelude::*;
use std::io::BufRead;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Sender};
use std::thread;

use crate::chunks::OrderedOutput;
use crate::{
    chunks::{Chunk, ChunkBuilder},
    elements::{ElementBuilder, ElementTypeBuilder, Member, Metadata, SimpleElementType},
    readers::Reader,
};

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

fn add_byte_field(field: &[u8], element_builder: &mut ElementBuilder) {
    let (flag, value) = field.split_at(1);
    macro_rules! value_as {
        ($type:ty) => {
            str_or_fail(value).parse::<$type>().unwrap()
        };
    }
    match flag {
        b"n" => {
            element_builder.id = Some(value_as!(i64));
        }
        b"w" => {
            element_builder.id = Some(value_as!(i64));
        }
        b"r" => {
            element_builder.id = Some(value_as!(i64));
        }
        b"v" => {
            element_builder.version = Some(value_as!(i32));
        }
        b"d" => match value {
            b"V" => element_builder.visible = Some(true),
            b"D" => element_builder.visible = Some(false),
            _ => {
                panic!("Deleted field value not recognized: {:?}", field);
            }
        },
        b"c" => {
            element_builder.changeset = Some(value_as!(i64));
        }
        b"t" => {
            element_builder.timestamp = Some(str_or_fail(value).to_owned());
        }
        b"i" => {
            element_builder.uid = Some(value_as!(i32));
        }
        b"u" => {
            element_builder.user = Some(str_or_fail(value).to_owned());
        }
        b"T" => {
            str_or_fail(value)
                .split(',')
                .filter_map(|t| t.split_once('='))
                .for_each(|(k, v)| {
                    element_builder
                        .tags
                        .insert(unescape_str(k), unescape_str(v));
                });
        }
        b"x" => match &mut element_builder.element_type {
            None => {
                element_builder.element_type = Some(ElementTypeBuilder::NodeBuilder {
                    lat: None,
                    lon: Some(value_as!(f64)),
                });
            }
            Some(ElementTypeBuilder::NodeBuilder { lon, .. }) => {
                *lon = Some(value_as!(f64));
            }
            _ => {
                panic!("Longitude set for a non-node element!");
            }
        },
        b"y" => match &mut element_builder.element_type {
            Some(ElementTypeBuilder::NodeBuilder { lat, .. }) => {
                *lat = Some(value_as!(f64));
            }
            None => {
                element_builder.element_type = Some(ElementTypeBuilder::NodeBuilder {
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

            element_builder.element_type = Some(ElementTypeBuilder::WayBuilder { nodes });
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
            element_builder.element_type = Some(ElementTypeBuilder::RelationBuilder { members })
        }
        _ => {
            panic!("Unrecognized field: {:?}", field);
        }
    }
}

fn convert_chunk(chunk: OrderedOutput<Box<[Vec<u8>]>>) -> Chunk {
    let mut elements = Vec::with_capacity(chunk.content.len());
    for line in chunk.content.iter() {
        let mut element_builder = ElementBuilder::default();
        let mut field_start = 0;
        for (i, &b) in line.iter().enumerate() {
            if b == b' ' {
                if field_start < i {
                    add_byte_field(&line[field_start..i], &mut element_builder);
                }
                field_start = i + 1;
            }
        }
        if field_start < line.len() {
            add_byte_field(&line[field_start..], &mut element_builder);
        }
        elements.push(element_builder.build());
    }
    Chunk {
        index: chunk.index,
        elements: elements.into_boxed_slice(),
    }
}

pub struct OplReader {}

impl OplReader {
    pub fn new() -> Self {
        OplReader {}
    }
}

impl Reader for OplReader {
    fn read_file(
        self,
        src: Option<PathBuf>,
        metadata_sender: Sender<Metadata>,
        chunk_builder: ChunkBuilder,
        write_thread: thread::JoinHandle<()>,
        final_iterator: impl Fn(Chunk) + Sync,
    ) {
        let (sender, receiver) = channel();
        // create an empty Metadata object
        let metadata = Metadata::default();
        metadata_sender
            .send(metadata)
            .expect("Couldn't send metadata to main thread!");

        let src = super::get_reader(src);
        let read_thread = thread::spawn(move || {
            src.split(b'\n')
                .map(|s| s.expect("Unable to read input file buffer"))
                .chunks(chunk_builder.max_size)
                .into_iter()
                .enumerate()
                .into_iter()
                .map(|(index, chunk)| OrderedOutput {
                    index,
                    content: chunk.collect::<Vec<Vec<u8>>>().into_boxed_slice(),
                })
                .for_each(|chunk| {
                    sender
                        .send(Box::new(chunk))
                        .expect("Unable to send chunk of vectors to channel");
                })
        });
        receiver
            .into_iter()
            .map(|chunk| convert_chunk(*chunk))
            .into_iter()
            .par_bridge()
            .for_each(|chunk| final_iterator(chunk));

        drop(final_iterator);

        write_thread
            .join()
            .expect("Couldn't join on write thread!!");

        read_thread.join().expect("Couldn't join on read thread!!");
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
