use std::sync::mpsc::Sender;
use serde_json::from_str;

use crate::elements;

pub fn read_json(sender: Sender<elements::Element>, metadata_sender: Sender<elements::Metadata>, src: &str) {
    let osm_json_object: elements::OsmDocument = match from_str(src) {
        Ok(v) => {
            eprintln!("Reading JSON input...");
            v
        },
        Err(e) => {
            panic!("ERROR: Could not parse JSON file: {e:?}");
        }
    };

    // send OSM document metadata to main thread
    metadata_sender.send(osm_json_object.metadata);

    // send each deserialized element to the next processing step
    for e in osm_json_object.elements {
        match sender.send(e) {
            Ok(_) => (),
            Err(e) => {
                panic!("ERROR: Unable to send an element: {e:?}");
            }
        }
    }
}
