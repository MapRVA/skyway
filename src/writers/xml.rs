use quick_xml::escape::escape;
use rayon::prelude::*;
use std::fmt::{Error, Write};
use std::sync::mpsc::{channel, Receiver};

use crate::{
    chunks::{Chunk, OrderedOutput, OrderedOutputIterator},
    elements::{Element, ElementType, Metadata, SimpleElementType},
    threadpools::WRITER_THREAD_POOL,
};

// wrapper struct that implements std::fmt::Write for any type
// that implements std::io::Write
struct ToFmtWrite<T>(pub T);

impl<T> Write for ToFmtWrite<T>
where
    T: std::io::Write,
{
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.0.write_all(s.as_bytes()).map_err(|_| std::fmt::Error)
    }
}

fn create_header(metadata: Metadata) -> String {
    let mut header = String::new();
    header.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<osm version=\"0.6\"");

    macro_rules! append_attribute {
        ($attr:ident) => {
            if let Some($attr) = &metadata.$attr {
                header.push_str(concat!(" ", stringify!($attr), "=\""));
                header.push_str(&escape($attr));
                header.push('\"');
            }
        };
    }

    append_attribute!(copyright);
    append_attribute!(generator);
    append_attribute!(license);
    append_attribute!(timestamp);
    append_attribute!(version);

    header.push_str(">\n");
    header
}

fn append_serialized_metadata(base: &mut String, element: &Element) {
    base.push_str(" id=\"");
    base.push_str(&lexical::to_string(element.id));
    base.push('\"');

    if let Some(c) = element.changeset {
        base.push_str(" changeset=\"");
        base.push_str(&lexical::to_string(c));
        base.push('\"');
    }

    if let Some(t) = &element.timestamp {
        base.push_str(" timestamp=\"");
        base.push_str(&escape(t));
        base.push('\"');
    }

    if let Some(u) = element.uid {
        base.push_str(" uid=\"");
        base.push_str(&lexical::to_string(u));
        base.push('\"');
    }

    if let Some(u) = &element.user {
        base.push_str(" user=\"");
        base.push_str(&escape(u));
        base.push('\"');
    }

    if element.visible == Some(true) {
        base.push_str(" visible=\"true\"");
    } else if element.visible == Some(false) {
        base.push_str(" visible=\"false\"");
    }
}

fn append_serialized_tags(base: &mut String, element: &Element) {
    for (k, v) in &element.tags {
        base.push_str("  <tag k=\"");
        base.push_str(&escape(k));
        base.push_str("\" v=\"");
        base.push_str(&escape(v));
        base.push_str("\"/>\n");
    }
}

fn append_serialized_element(base: &mut String, element: Element) {
    match &element.element_type {
        ElementType::Node { lat, lon } => {
            base.push_str(" <node lat=\"");
            base.push_str(&lexical::to_string(*lat));
            base.push_str("\" lon=\"");
            base.push_str(&lexical::to_string(*lon));
            base.push_str("\" ");

            append_serialized_metadata(base, &element);

            if element.tags.is_empty() {
                base.push('>');
                append_serialized_tags(base, &element);
                base.push_str(" </node>\n");
            } else {
                base.push_str("/>\n")
            }
        }
        ElementType::Way { nodes } => {
            // finish "type": "way", then start nodes dict
            base.push_str(" <way");
            append_serialized_metadata(base, &element);
            base.push_str(">\n");

            for n in nodes {
                base.push_str("  <nd ref=\"");
                base.push_str(&lexical::to_string(*n));
                base.push_str("\"/>\n")
            }

            append_serialized_tags(base, &element);

            base.push_str(" </way>\n");
        }
        ElementType::Relation { members } => {
            base.push_str(" <relation");
            append_serialized_metadata(base, &element);
            base.push_str(">\n");

            for m in members {
                base.push_str("  <member ");
                match m.t {
                    Some(SimpleElementType::Node) => base.push_str("type=\"node\""),
                    Some(SimpleElementType::Way) => base.push_str("type=\"way\""),
                    Some(SimpleElementType::Relation) => base.push_str("type=\"relation\""),
                    None => (),
                }

                base.push_str(" ref=\"");
                base.push_str(&lexical::to_string(m.id));
                base.push_str("\" role=\"");
                if let Some(ref r) = &m.role {
                    base.push_str(&escape(r));
                }
                base.push_str("\"/>\n");
            }
            base.push_str(" </relation>\n");
        }
    }
}

fn serialize_chunk(chunk: Chunk) -> Result<String, Error> {
    let mut output = String::with_capacity(chunk.elements.len() * 155);
    for element in chunk.elements {
        append_serialized_element(&mut output, element);
    }
    Ok(output)
}

pub fn write_xml<D: std::io::Write>(receiver: Receiver<Chunk>, metadata: Metadata, dest: D) {
    let mut writer = ToFmtWrite(dest);

    let (output_sender, output_receiver) = channel();
    WRITER_THREAD_POOL.install(move || {
        receiver
            .into_iter()
            .par_bridge()
            .map(|chunk| {
                let index = chunk.index;
                let content = serialize_chunk(chunk).expect("Failed to serialize chunk");
                OrderedOutput { index, content }
            })
            .for_each(|output| {
                output_sender
                    .send(output)
                    .expect("Failed to send serialized chunk");
            });
    });

    let header = create_header(metadata);
    writer
        .write_str(&header)
        .expect("Unable to write header to XML file!");

    let ordered_chunks = OrderedOutputIterator::new(output_receiver.into_iter());
    for chunk_content in ordered_chunks {
        writer
            .write_str(&chunk_content)
            .expect("Failed to write chunk");
    }

    writer
        .write_str("</osm>\n")
        .expect("Couldn't write final closing curly brace to output.");
}
