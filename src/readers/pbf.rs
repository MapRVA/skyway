use std::sync::mpsc::Sender;
use osmpbf;
use std::io::Read;
use std::collections::HashMap;

use crate::elements;

fn _get_tags(tag_iter: osmpbf::elements::TagIter) -> HashMap<String, String> {
    let mut tag_map = HashMap::new();
    let _ = tag_iter.map(|(k, v)| tag_map.insert(k.to_owned(), v.to_owned()));
    tag_map
}

fn _get_dense_tags(tag_iter: osmpbf::dense::DenseTagIter) -> HashMap<String, String> {
    let mut tag_map = HashMap::new();
    let _ = tag_iter.map(|(k, v)| tag_map.insert(k.to_owned(), v.to_owned()));
    tag_map
}

fn _convert_member(member: osmpbf::elements::RelMember) -> elements::Reference {
    elements::Reference {
        id: member.member_id,
        role: Some(member.role().unwrap().to_owned())
    }
}

fn _convert_element(element: osmpbf::Element) -> elements::Element {
    match element {
        osmpbf::Element::Node(node) => elements::Element {
            id: node.id(),
            version: None,
            tags: _get_tags(node.tags()),
            element_type: elements::ElementType::Node {
                latitude: node.lat(),
                longitude: node.lon(),
            },
        },
        osmpbf::Element::DenseNode(dense_node) => elements::Element {
            id: dense_node.id(),
            version: None,
            tags: _get_dense_tags(dense_node.tags()),
            element_type: elements::ElementType::Node {
                latitude: dense_node.lat(),
                longitude: dense_node.lon(),
            },
        },
        osmpbf::Element::Way(way) => elements::Element {
            id: way.id(),
            version: None,
            tags: _get_tags(way.tags()),
            element_type: elements::ElementType::Way {
                nodes: way.refs().collect(),
            },
        },
        osmpbf::Element::Relation(relation) => elements::Element {
            id: relation.id(),
            version: None,
            tags: _get_tags(relation.tags()),
            element_type: elements::ElementType::Relation {
                references: relation.members().map(_convert_member).collect(),
            },
        },
    } 
}

pub fn read_pbf<S: Read + Send>(sender: Sender<elements::Element>, src: S) {
    let reader = osmpbf::ElementReader::new(src);
    let element_count = reader.par_map_reduce(
        |element| {
            match sender.send(_convert_element(element)) {
                Ok(_) => 1,
                Err(_) => {
                    println!("ERROR: Unable to send an element.");
                    0
                }
            }
        },
        || 0_u64,
        |a, b| a + b
    );

    println!("Finished reading {element_count:?} elements from source.");
}
