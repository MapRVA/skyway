use std::collections::HashMap;

pub struct Version {
    pub version: u32,
    pub timestamp: i64,
    pub uid: Option<String>,
    pub user: Option<String>,
}

pub struct Reference {
    pub id: i64,
    pub role: Option<String>,
}

pub enum ElementType {
    Node {
        latitude: f64,
        longitude: f64,
    },
    Way {
        nodes: Vec<i64>,
    },
    Relation {
        references: Vec<Reference>,
    },
}

pub struct Element {
    pub id: i64,
    pub version: Option<Version>,
    pub tags: HashMap<String, String>,
    pub element_type: ElementType,
}
