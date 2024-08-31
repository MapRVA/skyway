use std::collections::HashMap;

#[derive(Debug)]
pub struct Member {
    pub t: Option<String>,
    pub id: i64,
    pub role: Option<String>,
}

#[derive(Debug)]
pub enum ElementType {
    Node { lat: f64, lon: f64 },
    Way { nodes: Vec<i64> },
    Relation { members: Vec<Member> },
}

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

#[derive(Debug)]
pub struct Metadata {
    pub version: Option<String>,
    pub generator: Option<String>,
    pub copyright: Option<String>,
    pub license: Option<String>,
}
