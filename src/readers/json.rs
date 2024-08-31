use serde::{Deserialize, Deserializer};
use serde_json::from_str;
use std::collections::HashMap;
use std::sync::mpsc::Sender;

use crate::elements::{Element, ElementType, Member, Metadata};

#[derive(Deserialize)]
#[serde(remote = "Member")]
struct MemberDef {
    #[serde(rename = "type")]
    t: Option<String>,
    #[serde(rename = "ref")]
    id: i64,
    role: Option<String>,
}

fn member_vec_annotation<'de, D>(deserializer: D) -> Result<Vec<Member>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct Wrapper(#[serde(with = "MemberDef")] Member);

    let v = Vec::deserialize(deserializer)?;
    Ok(v.into_iter().map(|Wrapper(a)| a).collect())
}

#[derive(Deserialize)]
#[serde(remote = "ElementType", tag = "type", rename_all = "lowercase")]
enum ElementTypeDef {
    Node {
        lat: f64,
        lon: f64,
    },
    Way {
        nodes: Vec<i64>,
    },
    Relation {
        #[serde(deserialize_with = "member_vec_annotation")]
        members: Vec<Member>,
    },
}

#[derive(Deserialize)]
#[serde(remote = "Element")]
struct ElementDef {
    changeset: Option<i64>,
    user: Option<String>,
    version: Option<i32>,
    uid: Option<i32>,
    id: i64,
    timestamp: Option<String>,
    visible: Option<bool>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    tags: HashMap<String, String>,
    #[serde(flatten, with = "ElementTypeDef")]
    element_type: ElementType,
}

#[derive(Deserialize)]
#[serde(remote = "Metadata")]
struct MetadataDef {
    version: Option<String>,
    generator: Option<String>,
    copyright: Option<String>,
    license: Option<String>,
}

#[derive(Deserialize)]
struct OsmDocument {
    #[serde(flatten, with = "MetadataDef")]
    metadata: Metadata,
    #[serde(deserialize_with = "element_vec_annotation")]
    elements: Vec<Element>,
}

fn element_vec_annotation<'de, D>(deserializer: D) -> Result<Vec<Element>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct Wrapper(#[serde(with = "ElementDef")] Element);

    let v = Vec::deserialize(deserializer)?;
    Ok(v.into_iter().map(|Wrapper(a)| a).collect())
}

pub fn read_json(sender: Sender<Element>, metadata_sender: Sender<Metadata>, src: &str) {
    let osm_json_object: OsmDocument = match from_str(src) {
        Ok(v) => {
            eprintln!("Reading JSON input...");
            v
        }
        Err(e) => {
            panic!("ERROR: Could not parse JSON file: {e:?}");
        }
    };

    // send OSM document metadata to main thread
    metadata_sender
        .send(osm_json_object.metadata)
        .expect("Couldn't send metdata to main thread!");

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
