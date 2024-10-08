//! A data structure for OpenStreetMap element data.

use std::collections::HashMap;

/// Element types without any additional metadata.
#[derive(Debug)]
pub enum SimpleElementType {
    Node,
    Way,
    Relation,
}

/// A member of a relation.
#[derive(Debug)]
pub struct Member {
    pub t: Option<SimpleElementType>,
    pub id: i64,
    pub role: Option<String>,
}

/// The varying characteristics of each element type.
#[derive(Debug)]
pub enum ElementType {
    Node { lat: f64, lon: f64 },
    Way { nodes: Vec<i64> },
    Relation { members: Vec<Member> },
}

/// An OpenStreetMap element.
#[derive(Debug)]
pub struct Element {
    pub changeset: Option<i64>,
    pub user: Option<String>,
    pub version: Option<i32>,
    pub uid: Option<i32>,
    pub id: i64,
    pub timestamp: Option<String>,
    pub visible: Option<bool>,
    pub tags: HashMap<String, String>,
    pub element_type: ElementType,
}

/// Document-level metadata.
#[derive(Debug, Default)]
pub struct Metadata {
    pub version: Option<String>,
    pub generator: Option<String>,
    pub copyright: Option<String>,
    pub license: Option<String>,
    pub timestamp: Option<String>,
}
