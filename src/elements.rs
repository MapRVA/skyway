use std::collections::HashMap;

pub trait Element {
    fn get_id(&self) -> &i64;
    fn get_tags(&self) -> &HashMap<String, String>;
    fn get_version(&self) -> &Option<Version>;
}

pub struct Reference {
    pub id: i64,
    pub role: Option<String>,
}

pub struct Version {
    pub version: u32,
    pub timestamp: i64,
    pub uid: Option<String>,
    pub user: Option<String>,
}

pub struct Relation {
    pub id: i64,
    pub version: Option<Version>,
    pub tags: HashMap<String, String>,
    pub references: Vec<Reference>,
}

impl Element for Relation {
    fn get_id(&self) -> &i64 {
        &self.id
    }
    fn get_tags(&self) -> &HashMap<String, String> {
        &self.tags
    }
    fn get_version(&self) -> &Option<Version> {
        &self.version
    }
}

pub struct Way {
    pub id: i64,
    pub version: Option<Version>,
    pub tags: HashMap<String, String>,
    pub nodes: Vec<i64>,
}

impl Element for Way {
    fn get_id(&self) -> &i64 {
        &self.id
    }
    fn get_tags(&self) -> &HashMap<String, String> {
        &self.tags
    }
    fn get_version(&self) -> &Option<Version> {
        &self.version
    }
}

pub struct Node {
    pub id: i64,
    pub version: Option<Version>,
    pub tags: HashMap<String, String>,
    pub latitude: f64,
    pub longitude: f64,
}

impl Element for Node {
    fn get_id(&self) -> &i64 {
        &self.id
    }
    fn get_tags(&self) -> &HashMap<String, String> {
        &self.tags
    }
    fn get_version(&self) -> &Option<Version> {
        &self.version
    }
}
