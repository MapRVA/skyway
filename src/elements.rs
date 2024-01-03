pub struct Tag {
    key: String,
    value: String,
}

pub struct Reference {
    id: u64,
    role: Option<String>,
}

pub struct Version {
    version: u32,
    timestamp: i64,
    uid: Option<String>,
    user: Option<String>,
}

pub struct Relation {
    id: u64,
    version: Option<Version>,
    tags: Vec<Tag>,
    references: Vec<Reference>,
}

pub struct Way {
    id: u64,
    version: Option<Version>,
    tags: Vec<Tag>,
    nodes: Vec<u64>,
}

pub struct Node {
    id: u64,
    version: Option<Version>,
    tags: Vec<Tag>,
    latitude: f64,
    longitude: f64,
}

pub enum Element {
    Node,
    Way,
    Relation,
}
