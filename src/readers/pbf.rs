use std::collections::HashMap;
use std::io::Read;
use std::sync::mpsc::Sender;

use crate::elements;

fn _get_tags(tag_iter: osmpbf::elements::TagIter) -> HashMap<String, String> {
    let mut tag_map = HashMap::new();
    for t in tag_iter {
        tag_map.insert(t.0.to_owned(), t.1.to_owned());
    }
    tag_map
}

fn _get_dense_tags(tag_iter: osmpbf::dense::DenseTagIter) -> HashMap<String, String> {
    let mut tag_map = HashMap::new();
    let _ = tag_iter.map(|(k, v)| tag_map.insert(k.to_owned(), v.to_owned()));
    tag_map
}

fn _convert_member(member: osmpbf::elements::RelMember) -> elements::Member {
    elements::Member {
        t: Some(match member.member_type {
            osmpbf::RelMemberType::Node => String::from("node"),
            osmpbf::RelMemberType::Way => String::from("way"),
            osmpbf::RelMemberType::Relation => String::from("relation"),
        }),
        id: member.member_id,
        role: Some(member.role().unwrap().to_owned()),
    }
}

fn _convert_element(element: osmpbf::Element) -> elements::Element {
    match element {
        osmpbf::Element::Node(node) => {
            let node_info = node.info();
            elements::Element {
                id: node.id(),
                tags: _get_tags(node.tags()),
                element_type: elements::ElementType::Node {
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
                elements::Element {
                    id: dense_node.id(),
                    tags: _get_dense_tags(dense_node.tags()),
                    element_type: elements::ElementType::Node {
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
                elements::Element {
                    id: dense_node.id(),
                    tags: _get_dense_tags(dense_node.tags()),
                    element_type: elements::ElementType::Node {
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
            elements::Element {
                id: way.id(),
                tags: _get_tags(way.tags()),
                element_type: elements::ElementType::Way {
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
            elements::Element {
                id: relation.id(),
                tags: _get_tags(relation.tags()),
                element_type: elements::ElementType::Relation {
                    members: relation.members().map(_convert_member).collect(),
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

pub fn read_pbf<S: Read + Send>(
    sender: Sender<elements::Element>,
    metadata_sender: Sender<elements::Metadata>,
    src: S,
) {
    metadata_sender
        .send(elements::Metadata {
            version: None,
            generator: None,
            copyright: None,
            license: None,
        })
        .expect("Couldn't send metdata to main thread!");
    eprintln!("Reading PBF input...");
    let reader = osmpbf::ElementReader::new(src);
    let element_count = reader.par_map_reduce(
        |element| match sender.send(_convert_element(element)) {
            Ok(_) => 1,
            Err(e) => {
                panic!("ERROR: Unable to send an element: {e:?}");
            }
        },
        || 0_u64,
        |a, b| a + b,
    );
    eprintln!("Finished reading {element_count:?} elements from source.");
}
