use quick_xml::de::from_str;
use serde::{Deserialize, Deserializer};
use serde_aux::field_attributes::{
    deserialize_bool_from_anything, deserialize_number_from_string,
    deserialize_option_number_from_string,
};
use std::collections::HashMap;
use std::sync::mpsc::Sender;

use crate::elements::{Element, ElementType, Member, Metadata};

#[derive(Deserialize)]
#[serde(remote = "Member", rename = "member")]
struct MemberDef {
    #[serde(rename = "@type")]
    t: Option<String>,
    #[serde(rename = "@ref", deserialize_with = "deserialize_number_from_string")]
    id: i64,
    #[serde(rename = "@role")]
    role: Option<String>,
}

#[derive(Deserialize)]
#[serde(remote = "Metadata")]
struct MetadataDef {
    #[serde(rename = "@version")]
    version: Option<String>,
    #[serde(rename = "@generator")]
    generator: Option<String>,
    #[serde(rename = "@copyright")]
    copyright: Option<String>,
    #[serde(rename = "@license")]
    license: Option<String>,
}

#[derive(Deserialize)]
struct XmlTags {
    #[serde(rename = "@k")]
    k: String,
    #[serde(rename = "@v")]
    v: String,
}

#[derive(Deserialize)]
pub struct XmlElementMeta {
    #[serde(rename = "@id", deserialize_with = "deserialize_number_from_string")]
    id: i64,
    #[serde(rename = "@user")]
    user: Option<String>,
    #[serde(
        rename = "@uid",
        deserialize_with = "deserialize_option_number_from_string"
    )]
    uid: Option<i32>,
    #[serde(
        rename = "@visible",
        deserialize_with = "deserialize_bool_from_anything"
    )]
    visible: bool,
    #[serde(
        rename = "@version",
        deserialize_with = "deserialize_option_number_from_string"
    )]
    version: Option<i32>,
    #[serde(
        rename = "@changeset",
        deserialize_with = "deserialize_option_number_from_string"
    )]
    changeset: Option<i64>,
    #[serde(rename = "@timestamp")]
    timestamp: Option<String>,
}

#[derive(Deserialize)]
struct XmlNode {
    #[serde(rename = "@lat", deserialize_with = "deserialize_number_from_string")]
    lat: f64,
    #[serde(rename = "@lon", deserialize_with = "deserialize_number_from_string")]
    lon: f64,
    #[serde(flatten)]
    meta: XmlElementMeta,
    #[serde(default, rename = "tag")]
    tags: Vec<XmlTags>,
}

#[derive(Deserialize)]
#[serde(rename = "nd")]
struct XmlWayNode {
    #[serde(rename = "@ref", deserialize_with = "deserialize_number_from_string")]
    nd_ref: i64,
}

#[derive(Deserialize)]
struct XmlWay {
    #[serde(flatten)]
    meta: XmlElementMeta,
    nd: Vec<XmlWayNode>,
    #[serde(default, rename = "tag")]
    tags: Vec<XmlTags>,
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
struct XmlRelation {
    #[serde(flatten)]
    meta: XmlElementMeta,
    #[serde(deserialize_with = "member_vec_annotation")]
    member: Vec<Member>,
    #[serde(default, rename = "tag")]
    tags: Vec<XmlTags>,
}

// for now, bounds are not converted
// #[derive(Deserialize)]
// struct Bounds {
//     #[serde(rename = "@minlat")]
//     minlat: String,
//     #[serde(rename = "@minlon")]
//     minlon: String,
//     #[serde(rename = "@maxlat")]
//     maxlat: String,
//     #[serde(rename = "@maxlon")]
//     maxlon: String,
// }

#[derive(Deserialize)]
#[serde(rename = "osm")]
struct OsmXmlDocument {
    #[serde(flatten, with = "MetadataDef")]
    metadata: Metadata,
    // bounds: Bounds,
    #[serde(default)]
    node: Vec<XmlNode>,
    #[serde(default)]
    way: Vec<XmlWay>,
    #[serde(default)]
    relation: Vec<XmlRelation>,
}

enum XmlElement {
    Node(XmlNode),
    Way(XmlWay),
    Relation(XmlRelation),
}

fn _convert_tags(xml_tags: Vec<XmlTags>) -> HashMap<String, String> {
    let mut tags_hashmap = HashMap::new();
    for tag in xml_tags {
        tags_hashmap.insert(tag.k, tag.v);
    }
    tags_hashmap
}

fn _convert_element(xml_element: XmlElement) -> Element {
    match xml_element {
        XmlElement::Node(node) => Element {
            changeset: node.meta.changeset,
            user: node.meta.user,
            version: node.meta.version,
            uid: node.meta.uid,
            id: node.meta.id,
            timestamp: node.meta.timestamp,
            visible: Some(node.meta.visible),
            tags: _convert_tags(node.tags),
            element_type: ElementType::Node {
                lat: node.lat,
                lon: node.lon,
            },
        },
        XmlElement::Way(way) => Element {
            changeset: way.meta.changeset,
            user: way.meta.user,
            version: way.meta.version,
            uid: way.meta.uid,
            id: way.meta.id,
            timestamp: way.meta.timestamp,
            visible: Some(way.meta.visible),
            tags: _convert_tags(way.tags),
            element_type: ElementType::Way {
                nodes: way.nd.iter().map(|n| n.nd_ref).collect(),
            },
        },
        XmlElement::Relation(rel) => Element {
            changeset: rel.meta.changeset,
            user: rel.meta.user,
            version: rel.meta.version,
            uid: rel.meta.uid,
            id: rel.meta.id,
            timestamp: rel.meta.timestamp,
            visible: Some(rel.meta.visible),
            tags: _convert_tags(rel.tags),
            element_type: ElementType::Relation {
                members: rel.member,
            },
        },
    }
}

pub fn read_xml(sender: Sender<Element>, metadata_sender: Sender<Metadata>, src: &str) {
    let osm_xml_object: OsmXmlDocument = match from_str(src) {
        Ok(v) => {
            eprintln!("Reading XML input...");
            v
        }
        Err(e) => {
            panic!("ERROR: Could not parse XML file: {e:?}");
        }
    };

    // send OSM document metadata to main thread
    metadata_sender
        .send(osm_xml_object.metadata)
        .expect("Couldn't send metdata to main thread!");

    // send each deserialized element to the next processing step
    for n in osm_xml_object.node {
        match sender.send(_convert_element(XmlElement::Node(n))) {
            Ok(_) => (),
            Err(e) => {
                panic!("ERROR: Unable to send a node: {e:?}");
            }
        }
    }
    for w in osm_xml_object.way {
        match sender.send(_convert_element(XmlElement::Way(w))) {
            Ok(_) => (),
            Err(e) => {
                panic!("ERROR: Unable to send a way: {e:?}");
            }
        }
    }
    for r in osm_xml_object.relation {
        match sender.send(_convert_element(XmlElement::Relation(r))) {
            Ok(_) => (),
            Err(e) => {
                panic!("ERROR: Unable to send a relation: {e:?}");
            }
        }
    }
}
