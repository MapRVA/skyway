use std::sync::mpsc::Sender;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::from_str;

use crate::elements;

#[derive(Debug, Deserialize, Serialize)]
struct JSONMember {
    #[serde(rename = "type")]
    t: String,
    #[serde(rename = "ref")]
    r: i64,
    role: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum JSONElement {
    Relation {
        id: i64,
        timestamp: String,
        version: i64,
        changeset: i64,
        user: String,
        uid: i64,
        members: Vec<JSONMember>,
        #[serde(default)]
        tags: HashMap<String, String>,
    },
    Way {
        id: i64,
        timestamp: String,
        version: i64,
        changeset: i64,
        user: String,
        uid: i64,
        nodes: Vec<i64>,
        #[serde(default)]
        tags: HashMap<String, String>,
    },
    Node {
        id: i64,
        lat: f64,
        lon: f64,
        timestamp: String,
        version: i64,
        changeset: i64,
        user: String,
        uid: i64,
        #[serde(default)]
        tags: HashMap<String, String>,
    },
}

#[derive(Debug, Deserialize, Serialize)]
struct OsmJson {
    version: String,
    generator: String,
    copyright: String,
    attribution: String,
    license: String,
    elements: Vec<JSONElement>,
}

fn _convert_element(element: JSONElement) -> elements::Element {
    match element {
        JSONElement::Node { id, lat, lon,      tags, .. } => elements::Element {
            id,
            version: None,
            tags,
            element_type: elements::ElementType::Node {
                latitude: lat,
                longitude: lon,
            },
        },
        JSONElement::Way { id,      nodes, tags, .. } => elements::Element {
            id,
            version: None,
            tags,
            element_type: elements::ElementType::Way {
                nodes,
            },
        },
        JSONElement::Relation { id,      members, tags, .. } => {
            let mut references = Vec::new();
            for r in members {
                references.push(elements::Reference {
                    id: r.r,
                    role: Some(r.role),
                })
            }
            elements::Element {
                id,
                version: None,
                tags,
                element_type: elements::ElementType::Relation {
                    references,
                },
            }
        },
    } 
}


pub fn read_json(sender: Sender<elements::Element>, src: &str) {
    let osm_json_object: OsmJson = match from_str(src) {
        Ok(v) => v,
        Err(e) => {
            panic!("Could not parse JSON file: {e:?}");
        }
    };
    for e in osm_json_object.elements {
        match sender.send(_convert_element(e)) {
            Ok(_) => (),
            Err(e) => {
                println!("ERROR: Unable to send an element: {e:?}");
            }
        }
    }
}
