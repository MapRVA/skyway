use osmpbf::{BlobDecode, BlobReader};
use rayon::prelude::*;
use std::collections::HashMap;
use std::io::{empty, Read};
use std::mem;
use std::sync::mpsc::Sender;

use crate::elements::{Element, ElementType, Member, Metadata, SimpleElementType};
use crate::readers::Reader;
use crate::threadpools::READER_THREAD_POOL;

fn get_tags(tag_iter: osmpbf::elements::TagIter) -> HashMap<String, String> {
    let mut tag_map = HashMap::new();
    for t in tag_iter {
        tag_map.insert(t.0.to_owned(), t.1.to_owned());
    }
    tag_map
}

fn get_dense_tags(tag_iter: osmpbf::dense::DenseTagIter) -> HashMap<String, String> {
    let mut tag_map = HashMap::new();
    let _ = tag_iter.map(|(k, v)| tag_map.insert(k.to_owned(), v.to_owned()));
    tag_map
}

fn convert_member(member: osmpbf::elements::RelMember) -> Member {
    Member {
        t: Some(match member.member_type {
            osmpbf::RelMemberType::Node => SimpleElementType::Node,
            osmpbf::RelMemberType::Way => SimpleElementType::Way,
            osmpbf::RelMemberType::Relation => SimpleElementType::Relation,
        }),
        id: member.member_id,
        role: Some(member.role().unwrap().to_owned()),
    }
}

fn convert_element(element: osmpbf::Element) -> Element {
    match element {
        osmpbf::Element::Node(node) => {
            let node_info = node.info();
            Element {
                id: node.id(),
                tags: get_tags(node.tags()),
                element_type: ElementType::Node {
                    lat: node.lat(),
                    lon: node.lon(),
                },
                changeset: node_info.changeset(),
                user: None, // TODO
                uid: node_info.uid(),
                timestamp: None, // TODO
                visible: Some(node_info.visible()),
                version: node_info.version(),
            }
        }
        osmpbf::Element::DenseNode(dense_node) => {
            if let Some(dense_node_info) = dense_node.info() {
                Element {
                    id: dense_node.id(),
                    tags: get_dense_tags(dense_node.tags()),
                    element_type: ElementType::Node {
                        lat: dense_node.lat(),
                        lon: dense_node.lon(),
                    },
                    changeset: Some(dense_node_info.changeset()),
                    user: None, // TODO
                    uid: Some(dense_node_info.uid()),
                    timestamp: None, // TODO
                    visible: Some(dense_node_info.visible()),
                    version: Some(dense_node_info.version()),
                }
            } else {
                Element {
                    id: dense_node.id(),
                    tags: get_dense_tags(dense_node.tags()),
                    element_type: ElementType::Node {
                        lat: dense_node.lat(),
                        lon: dense_node.lon(),
                    },
                    changeset: None,
                    user: None,
                    uid: None,
                    timestamp: None,
                    visible: None,
                    version: None,
                }
            }
        }
        osmpbf::Element::Way(way) => {
            let way_info = way.info();
            Element {
                id: way.id(),
                tags: get_tags(way.tags()),
                element_type: ElementType::Way {
                    nodes: way.refs().collect(),
                },
                changeset: way_info.changeset(),
                user: None, // TODO
                uid: way_info.uid(),
                timestamp: None, // TODO
                visible: Some(way_info.visible()),
                version: way_info.version(),
            }
        }
        osmpbf::Element::Relation(relation) => {
            let relation_info = relation.info();
            Element {
                id: relation.id(),
                tags: get_tags(relation.tags()),
                element_type: ElementType::Relation {
                    members: relation.members().map(convert_member).collect(),
                },
                changeset: relation_info.changeset(),
                user: None, // TODO
                uid: relation_info.uid(),
                timestamp: None, // TODO
                visible: Some(relation_info.visible()),
                version: relation_info.version(),
            }
        }
    }
}

pub struct PbfReader {
    pub src: Box<dyn Read + Send>,
}

impl Reader for PbfReader {
    fn read(&mut self, sender: Sender<Vec<Element>>, metadata_sender: Sender<Metadata>) {
        metadata_sender
            .send(Metadata {
                version: None,
                generator: None,
                copyright: None,
                license: None,
                timestamp: None, // TODO: see if this is available?
            })
            .expect("Couldn't send metdata to main thread!");

        let src = mem::replace(&mut self.src, Box::new(empty()));
        let reader = BlobReader::new(src);
        READER_THREAD_POOL.install(|| {
            reader
                .par_bridge()
                .filter_map(|blob| match blob.unwrap().decode() {
                    Ok(BlobDecode::OsmData(block)) => {
                        Some(block.elements().map(convert_element).collect())
                    }
                    Ok(BlobDecode::OsmHeader(_)) | Ok(BlobDecode::Unknown(_)) => None,
                    Err(e) => panic!("ERROR: unable to read PBF input: {e:?}"),
                })
                .for_each(|b| {
                    sender
                        .send(b)
                        .expect("Unable to send chunk of elements to channel.")
                })
        });
    }
}
