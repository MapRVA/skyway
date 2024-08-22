use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize)]
pub struct Member {
    #[serde(rename = "type")]
    pub t: Option<String>,
    #[serde(alias = "ref")]
    pub id: i64,
    pub role: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
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

#[derive(Debug, Deserialize, Serialize)]
pub struct Element {
    pub changeset: Option<i64>,
    pub user: Option<String>,
    pub version: Option<i32>,
    pub uid: Option<i32>,
    pub id: i64,
    pub timestamp: Option<String>,
    #[serde(default)]
    pub tags: HashMap<String, String>,
    #[serde(flatten)]
    pub element_type: ElementType,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OsmDocument {
    pub version: Option<String>,
    pub generator: Option<String>,
    pub copyright: Option<String>,
    pub license: Option<String>,
    pub elements: Vec<Element>,
}
