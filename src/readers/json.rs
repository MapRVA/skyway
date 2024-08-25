use std::sync::mpsc::Sender;
use serde_json::from_str;
use serde::Deserialize;
use rename_item::rename;
use std::collections::HashMap;

use crate::elements::{Element, Metadata, ElementType, Member};

#[derive(Deserialize)]
#[rename(name = "member-def")]
#[allow(dead_code)]
pub struct Member {
    #[serde(rename = "type")]
    pub t: Option<String>,
    #[serde(rename = "ref")]
    pub id: i64,
    pub role: Option<String>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
#[rename(name = "element-type-def")]
#[allow(dead_code)]
pub enum ElementType {
    Node {
        lat: f64,
        lon: f64,
    },
    Way {
        nodes: Vec<i64>,
    },
    Relation {
        members: Vec<Member>,
    },
}

#[derive(Deserialize)]
#[rename(name = "element-def")]
#[allow(dead_code)]
pub struct Element {
    pub changeset: Option<i64>,
    pub user: Option<String>,
    pub version: Option<i32>,
    pub uid: Option<i32>,
    pub id: i64,
    pub timestamp: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub tags: HashMap<String, String>,
    #[serde(flatten)]
    pub element_type: ElementType,
}

#[derive(Deserialize)]
#[rename(name = "metadata-def")]
#[allow(dead_code)]
pub struct Metadata {
    pub version: Option<String>,
    pub generator: Option<String>,
    pub copyright: Option<String>,
    pub license: Option<String>,
}

#[derive(Deserialize)]
struct OsmDocument {
    #[serde(flatten)]
    metadata: Metadata,
    elements: Vec<Element>,
}

pub fn read_json(sender: Sender<Element>, metadata_sender: Sender<Metadata>, src: &str) {
    let osm_json_object: OsmDocument = match from_str(src) {
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
