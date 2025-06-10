use quick_xml::escape::escape;

use std::{
    fmt::Write,
    fs::File,
    io::stdout,
    path::PathBuf,
    sync::mpsc::{Receiver, channel},
};

use crate::{
    SkywayError,
    chunks::{Chunk, ElementChunk, OrderedChunkIterator},
    elements::{Element, ElementType, Metadata, SimpleElementType},
};

use super::Writer;

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
    header.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<osm");

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

    if let Some(v) = element.version {
        base.push_str(" version=\"");
        base.push_str(&lexical::to_string(v));
        base.push('\"');
    }

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
            base.push_str("\"");

            append_serialized_metadata(base, &element);

            if element.tags.is_empty() {
                base.push_str("/>\n");
            } else {
                base.push_str(">\n");
                append_serialized_tags(base, &element);
                base.push_str(" </node>\n");
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
                if let Some(r) = &m.role {
                    base.push_str(&escape(r));
                }
                base.push_str("\"/>\n");
            }

            append_serialized_tags(base, &element);

            base.push_str(" </relation>\n");
        }
    }
}

fn serialize_chunk(chunk: ElementChunk) -> Chunk<String> {
    let mut output = String::with_capacity(chunk.content.len() * 155);
    for element in chunk.content {
        append_serialized_element(&mut output, element);
    }
    Chunk {
        index: chunk.index,
        content: output,
    }
}

fn write_output(
    metadata_receiver: Receiver<Metadata>,
    data_receiver: Receiver<Chunk<String>>,
    dest: impl std::io::Write,
) {
    let metadata = metadata_receiver.into_iter().next();
    let mut writer = ToFmtWrite(dest);
    let header = create_header(metadata.unwrap()); // TODO: better error message if this unexpectedly panics
    writer
        .write_str(&header)
        .expect("Unable to write header to XML file!");

    let ordered_chunks = OrderedChunkIterator::new(data_receiver.into_iter());
    for chunk_content in ordered_chunks {
        writer
            .write_str(&chunk_content)
            .expect("Failed to write chunk");
    }

    writer
        .write_str("</osm>\n")
        .expect("Couldn't write final closing curly brace to output.");
}

pub struct XmlWriter {}

impl XmlWriter {
    pub fn new() -> Self {
        XmlWriter {}
    }
}

impl Writer for XmlWriter {
    fn write(
        &self,
        element_receiver: Receiver<ElementChunk>,
        metadata_receiver: Receiver<Metadata>,
        dest: Option<PathBuf>,
    ) -> Result<(), SkywayError> {
        let (sender, receiver) = channel();
        let write_thread = std::thread::spawn({
            move || {
                match dest {
                    None => write_output(metadata_receiver, receiver, stdout()),
                    Some(a) => match File::create(PathBuf::from(a)) {
                        Ok(b) => write_output(metadata_receiver, receiver, b),
                        Err(e) => {
                            panic!("Unable to open output file: {e:?}");
                        }
                    },
                };
            }
        });

        for chunk in element_receiver {
            sender
                .send(serialize_chunk(chunk))
                .expect("Failed to send serialized chunk");
        }

        drop(sender);

        write_thread.join().map_err(|e| {
            SkywayError::UnexpectedError(format!("Could not join writer thread: {:?}", e))
        })?;

        Ok(())
    }
}
