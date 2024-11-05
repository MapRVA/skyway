//! A data structure for OpenStreetMap element data.

use ustr::{Ustr, UstrMap};

/// Element types without any additional metadata.
#[derive(Debug, PartialEq)]
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
    Node { lat: f64, lon: f64 },
    Way { nodes: Vec<i64> },
    Relation { members: Vec<Member> },
}

/// An OpenStreetMap element.
#[derive(Debug, PartialEq)]
pub struct Element {
    pub changeset: Option<i64>,
    pub user: Option<Ustr>,
    pub version: Option<i32>,
    pub uid: Option<i32>,
    pub id: i64,
    pub timestamp: Option<String>,
    pub visible: Option<bool>,
    pub tags: UstrMap<String>,
    pub element_type: ElementType,
}

/// Builder type for ElementType, must be used with ElementBuilder.
#[derive(Debug, PartialEq)]
pub enum ElementTypeBuilder {
    NodeBuilder { lat: Option<f64>, lon: Option<f64> },
    WayBuilder { nodes: Vec<i64> },
    RelationBuilder { members: Vec<Member> },
}

/// Builder type for Element, used to construct Elements iteratively.
#[derive(Debug, Default, PartialEq)]
pub struct ElementBuilder {
    pub changeset: Option<i64>,
    pub user: Option<Ustr>,
    pub version: Option<i32>,
    pub uid: Option<i32>,
    pub id: Option<i64>,
    pub timestamp: Option<String>,
    pub visible: Option<bool>,
    pub tags: UstrMap<String>,
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
