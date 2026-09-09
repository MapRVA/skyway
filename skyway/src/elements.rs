//! A data structure for OpenStreetMap element data.

use std::collections::HashMap;

/// Element types without any additional metadata.
#[derive(Debug, PartialEq, Clone)]
pub enum SimpleElementType {
    Node,
    Way,
    Relation,
}

/// A member of a relation.
#[derive(Debug, PartialEq)]
pub struct Member {
    pub t: Option<SimpleElementType>,
    pub id: i64,
    pub role: Option<String>,
}

/// The varying characteristics of each element type.
#[derive(Debug, PartialEq)]
pub enum ElementType {
    Node { lat: i32, lon: i32 },
    Way { nodes: Vec<i64> },
    Relation { members: Vec<Member> },
}

/// An OpenStreetMap element.
#[derive(Debug, PartialEq)]
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

/// Identity of an element in a snapshot dataset: one element per (type, ID).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ElementKey {
    Node(i64),
    Way(i64),
    Relation(i64),
}

impl ElementKey {
    pub fn new(element_type: &SimpleElementType, id: i64) -> Self {
        match element_type {
            SimpleElementType::Node => ElementKey::Node(id),
            SimpleElementType::Way => ElementKey::Way(id),
            SimpleElementType::Relation => ElementKey::Relation(id),
        }
    }

    pub fn id(self) -> i64 {
        match self {
            ElementKey::Node(id) | ElementKey::Way(id) | ElementKey::Relation(id) => id,
        }
    }
}

impl std::fmt::Display for ElementKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ElementKey::Node(id) => write!(f, "node {id}"),
            ElementKey::Way(id) => write!(f, "way {id}"),
            ElementKey::Relation(id) => write!(f, "relation {id}"),
        }
    }
}

impl Member {
    /// The member's identity, if its type is known.
    ///
    /// A bare ID is ambiguous across element types, so an untyped member
    /// cannot be resolved to a particular element.
    pub fn key(&self) -> Option<ElementKey> {
        self.t.as_ref().map(|t| ElementKey::new(t, self.id))
    }
}

impl Element {
    /// The element's type, without its type-specific data.
    pub fn simple_type(&self) -> SimpleElementType {
        match self.element_type {
            ElementType::Node { .. } => SimpleElementType::Node,
            ElementType::Way { .. } => SimpleElementType::Way,
            ElementType::Relation { .. } => SimpleElementType::Relation,
        }
    }

    /// The element's identity.
    pub fn key(&self) -> ElementKey {
        ElementKey::new(&self.simple_type(), self.id)
    }

    /// Keys of every element this element references directly.
    ///
    /// Member types are preserved. A relation member without a type cannot be
    /// resolved, because a bare ID is ambiguous across element types, so it
    /// is reported as an error.
    pub fn reference_keys(&self) -> Result<Vec<ElementKey>, crate::SkywayError> {
        match &self.element_type {
            ElementType::Node { .. } => Ok(Vec::new()),
            ElementType::Way { nodes } => Ok(nodes.iter().map(|id| ElementKey::Node(*id)).collect()),
            ElementType::Relation { members } => members
                .iter()
                .map(|member| {
                    member.key().ok_or_else(|| {
                        crate::SkywayError::InvalidInputFile(format!(
                            "relation {} has a member with ID {} but no type, so its references cannot be resolved",
                            self.id, member.id
                        ))
                    })
                })
                .collect(),
        }
    }
}

/// Builder type for ElementType, must be used with ElementBuilder.
#[derive(Debug, PartialEq)]
pub enum ElementTypeBuilder {
    NodeBuilder { lat: Option<i32>, lon: Option<i32> },
    WayBuilder { nodes: Vec<i64> },
    RelationBuilder { members: Vec<Member> },
}

/// Builder type for Element, used to construct Elements iteratively.
#[derive(Debug, Default, PartialEq)]
pub struct ElementBuilder {
    pub changeset: Option<i64>,
    pub user: Option<String>,
    pub version: Option<i32>,
    pub uid: Option<i32>,
    pub id: Option<i64>,
    pub timestamp: Option<String>,
    pub visible: Option<bool>,
    pub tags: HashMap<String, String>,
    pub element_type: Option<ElementTypeBuilder>,
}

impl ElementBuilder {
    /// Construct an Element
    pub fn build(self) -> Element {
        let element_type = match self.element_type {
            Some(ElementTypeBuilder::NodeBuilder { lat, lon }) => ElementType::Node {
                lat: lat.expect("Cannot build node without lat value"),
                lon: lon.expect("Cannot build node without lon value"),
            },
            Some(ElementTypeBuilder::WayBuilder { nodes }) => ElementType::Way { nodes },
            Some(ElementTypeBuilder::RelationBuilder { members }) => {
                ElementType::Relation { members }
            }
            None => panic!("An element cannot be build without a type"),
        };

        Element {
            changeset: self.changeset,
            user: self.user,
            version: self.version,
            uid: self.uid,
            id: self.id.expect("An element cannot be built without an id"),
            timestamp: self.timestamp,
            visible: self.visible,
            tags: self.tags,
            element_type,
        }
    }
}

/// Document-level metadata.
#[derive(Debug, Default, PartialEq)]
pub struct Metadata {
    pub version: Option<String>,
    pub generator: Option<String>,
    pub copyright: Option<String>,
    pub license: Option<String>,
    pub timestamp: Option<String>,
}
