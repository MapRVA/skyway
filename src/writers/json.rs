use json::stringify;
use lexical;
use rayon::prelude::*;
use std::fmt::{Error, Write};
use std::sync::mpsc::{channel, Receiver};

use crate::elements::{Element, ElementType, Metadata, SimpleElementType};
use crate::threadpools::WRITER_THREAD_POOL;

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
    let mut header = String::from("{");

    if let Some(c) = metadata.copyright {
        header.push_str("\"copyright\":");
        header.push_str(&stringify(c));
        header.push(',');
    }

    // TODO: attribution as well as copyright?

    if let Some(l) = metadata.license {
        header.push_str("\"license\":");
        header.push_str(&stringify(l));
        header.push(',');
    }

    // TODO: add skyway details to this?
    if let Some(g) = metadata.generator {
        header.push_str("\"generator\":");
        header.push_str(&stringify(g));
        header.push(',');
    }

    header.push_str("\"version\":\"0.6\",\"elements\":[");

    header
}

fn append_serialized_element(base: &mut String, element: Element) {
    // start this node
    base.push_str("{\"type\":");

    // take care of type-specific attributes
    // (dict order doesn't matter in JSON)
    match element.element_type {
        ElementType::Node { lat, lon } => {
            base.push_str("\"node\"");

            base.push_str(",\"lat\":");
            base.push_str(&lexical::to_string(lat));

            base.push_str(",\"lon\":");
            base.push_str(&lexical::to_string(lon));
        }
        ElementType::Way { nodes } => {
            // finish "type": "way", then start nodes dict
            base.push_str("\"way\",\"nodes\":[");

            // fill in nodes dict
            let mut first_node_appended = false;
            for n in nodes {
                if first_node_appended {
                    base.push(',');
                }
                first_node_appended = true;
                base.push_str(&lexical::to_string(n));
            }
            // close nodes dict
            base.push(']');
        }
        ElementType::Relation { members } => {
            // finish "type": "relation", then start members list
            base.push_str("\"relation\",\"members\":[");

            // fill in members list
            let mut first_node_appended = false;
            for m in members {
                if first_node_appended {
                    base.push(',');
                }
                first_node_appended = true;

                base.push('{');

                base.push_str("\"ref\":");
                base.push_str(&lexical::to_string(m.id));

                match m.t {
                    Some(SimpleElementType::Node) => base.push_str(",\"type\":\"node\""),
                    Some(SimpleElementType::Way) => base.push_str(",\"type\":\"way\""),
                    Some(SimpleElementType::Relation) => base.push_str(",\"type\":\"relation\""),
                    None => (),
                }

                base.push_str(",\"role\":");
                match m.role {
                    Some(r) => base.push_str(&stringify(r)),
                    None => base.push_str("\"\""),
                }

                base.push('}');
            }
            base.push(']');
        }
    }

    base.push_str(",\"id\":");
    base.push_str(&lexical::to_string(element.id));

    if let Some(c) = element.changeset {
        base.push_str(",\"changeset\":");
        base.push_str(&lexical::to_string(c));
    }

    if let Some(t) = element.timestamp {
        base.push_str(",\"timestamp\":");
        base.push_str(&stringify(t));
    }

    if let Some(u) = element.uid {
        base.push_str(",\"uid\":");
        base.push_str(&lexical::to_string(u));
    }

    if let Some(u) = element.user {
        base.push_str(",\"user\":");
        base.push_str(&stringify(u));
    }

    // add visible field only if it is false
    if element.visible == Some(false) {
        base.push_str(",\"visible\":false");
    }

    // append this element's tags to base
    if !element.tags.is_empty() {
        base.push_str(",\"tags\":{");
        let mut first_tag_appended = false;
        for (k, v) in element.tags {
            if first_tag_appended {
                base.push(',');
            }
            first_tag_appended = true;
            base.push_str(&stringify(k));
            base.push(':');
            base.push_str(&stringify(v));
        }
        base.push('}');
    }

    // finish element
    base.push('}');
}

fn serialize_chunk(chunk: Vec<Element>) -> Result<String, Error> {
    let mut output = String::new();
    let mut first_element_appended = false;
    for element in chunk {
        if first_element_appended {
            output.push(',');
        }
        first_element_appended = true;
        append_serialized_element(&mut output, element);
    }
    Ok(output)
}

pub fn write_json<D: std::io::Write>(
    receiver: Receiver<Vec<Element>>,
    metadata: Metadata,
    dest: D,
) {
    let mut writer = ToFmtWrite(dest);

    // TODO: append metadata to output

    let (output_sender, output_reciever) = channel();
    WRITER_THREAD_POOL.install(move || {
        receiver
            .into_iter()
            .par_bridge()
            .map(serialize_chunk)
            .map(|result| result.expect("Failed to serialize chunk"))
            .for_each(|s| match output_sender.clone().send(s) {
                Ok(_) => (),
                Err(e) => panic!("Error passing output chunk between threads: {e:?}"),
            });
    });

    let header = create_header(metadata);

    writer
        .write_str(&header)
        .expect("Couldn't write opening metadata to output.");

    for output_string in output_reciever {
        writer
            .write_str(&output_string)
            .expect("Failed to write to output");
    }

    writer
        .write_str("]}")
        .expect("Couldn't write final closing curly brace to output.");
}
