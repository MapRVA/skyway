use std::sync::mpsc::Receiver;
use serde_json::to_writer;
use std::io::Write;
use std::collections::HashMap;
use serde::{Serialize, Serializer};

use crate::elements::{Element, Metadata, Member, ElementType};

#[derive(Serialize)]
#[serde(remote = "Member")]
pub struct MemberDef {
    #[serde(rename = "type")]
    pub t: Option<String>,
    #[serde(rename = "ref")]
    pub id: i64,
    pub role: Option<String>,
}

#[derive(Serialize)]
#[serde(remote = "ElementType", tag = "type", rename_all = "lowercase")]
pub enum ElementTypeDef {
    Node {
        lat: f64,
        lon: f64,
    },
    Way {
        nodes: Vec<i64>,
    },
    Relation {
        #[serde(serialize_with = "serialize_member_vec")]
        members: Vec<Member>,
    },
}

fn serialize_member_vec<S: Serializer>(v: &[Member], serializer: S) -> Result<S::Ok, S::Error> {
    #[derive(Serialize)]
    struct Wrapper<'a>(#[serde(with = "MemberDef")] &'a Member);

    v.iter()
        .map(Wrapper)
        .collect::<Vec<_>>()
        .serialize(serializer)
}


fn _skip_visibility(visibility: &Option<bool>) -> bool {
    visibility.unwrap_or(true)
}

#[derive(Serialize)]
#[serde(remote = "Element")]
pub struct ElementDef {
    pub changeset: Option<i64>,
    pub user: Option<String>,
    pub version: Option<i32>,
    pub uid: Option<i32>,
    pub id: i64,
    pub timestamp: Option<String>,
    #[serde(skip_serializing_if = "_skip_visibility")]
    pub visible: Option<bool>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub tags: HashMap<String, String>,
    #[serde(flatten, with = "ElementTypeDef")]
    pub element_type: ElementType,
}

#[derive(Serialize)]
#[serde(remote = "Metadata")]
pub struct MetadataDef {
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
    #[serde(flatten, with = "MetadataDef")]
    pub metadata: Metadata,
    #[serde(serialize_with = "serialize_element_vec")]
    pub elements: Vec<Element>,
}

fn serialize_element_vec<S: Serializer>(v: &[Element], serializer: S) -> Result<S::Ok, S::Error> {
    #[derive(Serialize)]
    struct Wrapper<'a>(#[serde(with = "ElementDef")] &'a Element);

    v.iter()
        .map(Wrapper)
        .collect::<Vec<_>>()
        .serialize(serializer)
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
