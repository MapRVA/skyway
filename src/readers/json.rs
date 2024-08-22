use std::sync::mpsc::Sender;
use serde_json::from_str;

use crate::elements;

pub fn read_json(sender: Sender<elements::Element>, src: &str) {
    let osm_json_object: elements::OsmDocument = match from_str(src) {
        Ok(v) => v,
        Err(e) => {
            panic!("ERROR: Could not parse JSON file: {e:?}");
        }
    };
    for e in osm_json_object.elements {
        match sender.send(e) {
            Ok(_) => (),
            Err(e) => {
                println!("ERROR: Unable to send an element: {e:?}");
            }
        }
    }
}
