use std::sync::mpsc::Receiver;
use serde_json::to_writer;
use std::io::Write;

use crate::elements;

pub fn write_json<D: Write>(receiver: Receiver<elements::Element>, metadata: elements::Metadata, dest: D) {
    let mut received_elements = Vec::new();
    for e in receiver {
        received_elements.push(e);
    }

    let osm_document = elements::OsmDocument {
        metadata,
        elements: received_elements,
    };

    match to_writer(dest, &osm_document) {
        Ok(_) => {
            eprintln!("Successfully wrote output.");
        },
        Err(e) => {
            panic!("JSON serialization error: {e:?}");
        },
    }
}
