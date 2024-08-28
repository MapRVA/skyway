use std::sync::mpsc::Receiver;
use serde_json::to_writer;
use std::io::Write;
use std::collections::HashMap;
use serde::Serialize;
use rename_item::rename;

use crate::elements::{Element, Metadata, Member, ElementType};

#[derive(Serialize)]
#[rename(name = "member-def")]
#[allow(dead_code)]
pub struct Member {
    #[serde(rename = "type")]
    pub t: Option<String>,
    #[serde(rename = "ref")]
    pub id: i64,
    pub role: Option<String>,
}

#[derive(Serialize)]
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

#[derive(Serialize)]
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

#[derive(Serialize)]
#[rename(name = "metadata-def")]
#[allow(dead_code)]
pub struct Metadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copyright: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
}

#[derive(Serialize)]
pub struct OsmDocument {
    #[serde(flatten)]
    pub metadata: Metadata,
    pub elements: Vec<Element>,
}

pub fn write_json<D: Write>(receiver: Receiver<Element>, metadata: Metadata, dest: D) {
    let mut received_elements = Vec::new();
    for e in receiver {
        received_elements.push(e);
    }

    let osm_document = OsmDocument {
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
