use quick_xml::se::to_writer_with_root;
use serde::{Serialize, Serializer};
use std::collections::HashMap;
use std::fmt::Write;
use std::sync::mpsc::Receiver;

use crate::elements::{Element, ElementType, Member, Metadata};

#[derive(Serialize)]
#[serde(remote = "Member")]
struct MemberDef {
    #[serde(rename = "@type")]
    t: Option<String>,
    #[serde(rename = "@ref")]
    id: i64,
    #[serde(rename = "@role")]
    role: Option<String>,
}

#[derive(Serialize)]
struct XmlTags {
    #[serde(rename = "@k")]
    k: String,
    #[serde(rename = "@v")]
    v: String,
}

#[derive(Serialize)]
pub struct XmlElementMeta {
    #[serde(rename = "@id")]
    id: i64,
    #[serde(rename = "@user")]
    user: Option<String>,
    #[serde(rename = "@uid")]
    uid: Option<i32>,
    #[serde(rename = "@visible")]
    visible: bool,
    #[serde(rename = "@version")]
    version: Option<i32>,
    #[serde(rename = "@changeset")]
    changeset: Option<i64>,
    #[serde(rename = "@timestamp")]
    timestamp: Option<String>,
}

#[derive(Serialize)]
struct XmlNode {
    #[serde(rename = "@lat")]
    lat: f64,
    #[serde(rename = "@lon")]
    lon: f64,
    #[serde(flatten)]
    meta: XmlElementMeta,
    #[serde(default, rename = "tag")]
    tags: Vec<XmlTags>,
}

#[derive(Serialize)]
#[serde(rename = "nd")]
struct XmlWayNode {
    #[serde(rename = "@ref")]
    nd_ref: i64,
}

#[derive(Serialize)]
struct XmlWay {
    #[serde(flatten)]
    meta: XmlElementMeta,
    nd: Vec<XmlWayNode>,
    #[serde(default, rename = "tag")]
    tags: Vec<XmlTags>,
}

fn serialize_member_vec<S: Serializer>(v: &[Member], serializer: S) -> Result<S::Ok, S::Error> {
    #[derive(Serialize)]
    struct Wrapper<'a>(#[serde(with = "MemberDef")] &'a Member);

    v.iter()
        .map(Wrapper)
        .collect::<Vec<_>>()
        .serialize(serializer)
}

#[derive(Serialize)]
struct XmlRelation {
    #[serde(flatten)]
    meta: XmlElementMeta,
    #[serde(serialize_with = "serialize_member_vec")]
    member: Vec<Member>,
    #[serde(default, rename = "tag")]
    tags: Vec<XmlTags>,
}

#[derive(Serialize)]
pub struct XmlMetadata {
    #[serde(rename = "@version", skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(rename = "@generator", skip_serializing_if = "Option::is_none")]
    pub generator: Option<String>,
    #[serde(rename = "@copyright", skip_serializing_if = "Option::is_none")]
    pub copyright: Option<String>,
    #[serde(rename = "@license", skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
}

#[derive(Serialize)]
struct OsmXmlDocument {
    #[serde(flatten)]
    metadata: XmlMetadata,
    #[serde(default)]
    node: Vec<XmlNode>,
    #[serde(default)]
    way: Vec<XmlWay>,
    #[serde(default)]
    relation: Vec<XmlRelation>,
}

struct ToFmtWrite<T>(pub T);

impl<T> Write for ToFmtWrite<T>
where
    T: std::io::Write,
{
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.0.write_all(s.as_bytes()).map_err(|_| std::fmt::Error)
    }
}

fn _convert_tags(element_tags: HashMap<String, String>) -> Vec<XmlTags> {
    element_tags
        .into_iter()
        .map(|(k, v)| XmlTags { k, v })
        .collect()
}

fn _convert_nodes(way_nodes: Vec<i64>) -> Vec<XmlWayNode> {
    way_nodes
        .into_iter()
        .map(|nd_ref| XmlWayNode { nd_ref })
        .collect()
}

fn _split_and_convert_elements<I>(
    received_elements: I,
) -> (Vec<XmlNode>, Vec<XmlWay>, Vec<XmlRelation>)
where
    I: Iterator<Item = Element>,
{
    let mut nodes = Vec::new();
    let mut ways = Vec::new();
    let mut relations = Vec::new();
    for e in received_elements {
        let meta = XmlElementMeta {
            id: e.id,
            user: e.user,
            uid: e.uid,
            visible: e.visible.unwrap_or(true), // TODO: better default behavior?
            version: e.version,
            changeset: e.changeset,
            timestamp: e.timestamp,
        };
        let tags = _convert_tags(e.tags);
        match e.element_type {
            ElementType::Node { lat, lon } => nodes.push(XmlNode {
                lat,
                lon,
                meta,
                tags,
            }),
            ElementType::Way { nodes } => ways.push(XmlWay {
                meta,
                nd: _convert_nodes(nodes),
                tags,
            }),
            ElementType::Relation { members } => relations.push(XmlRelation {
                meta,
                member: members,
                tags,
            }),
        }
    }
    (nodes, ways, relations)
}

pub fn write_xml<D: std::io::Write>(receiver: Receiver<Element>, metadata: Metadata, dest: D) {
    let (node, way, relation) = _split_and_convert_elements(receiver.iter());

    let xml_osm_document = OsmXmlDocument {
        metadata: XmlMetadata {
            version: metadata.version,
            generator: metadata.generator,
            copyright: metadata.copyright,
            license: metadata.license,
        },
        node,
        way,
        relation,
    };

    let mut writer = ToFmtWrite(dest);

    writer
        .write_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>")
        .expect("Unable to write header to XML file!");

    match to_writer_with_root(writer, "osm", &xml_osm_document) {
        Ok(_) => {
            eprintln!("Successfully wrote output.");
        }
        Err(e) => {
            panic!("XML serialization error: {e:?}");
        }
    }
}
