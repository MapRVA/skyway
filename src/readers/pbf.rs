use chrono::{DateTime, SecondsFormat};
use osmpbf::{BlobDecode, BlobReader, HeaderBlock};
use rayon::prelude::*;

use std::{collections::HashMap, path::PathBuf, sync::mpsc::Sender};

use crate::{
    chunks::{ChunkBuilder, ElementChunk},
    elements::{Element, ElementType, Member, Metadata, SimpleElementType},
    readers::Reader,
    SkywayError,
};

/// Convert the OSM PBF timestamps to RFC 3339
fn convert_timestamp(milli_timestamp: i64) -> Result<String, SkywayError> {
    DateTime::from_timestamp_millis(milli_timestamp)
        .map_or(Err(SkywayError::InvalidInputFile), |d| {
            Ok(d.to_rfc3339_opts(SecondsFormat::Secs, true))
        })
}

fn timestamp_conversion_wrapper(timestamp: Option<i64>) -> Option<String> {
    timestamp.and_then(|t| {
        Some(convert_timestamp(t).expect("Could not convert timestamp from PBF file."))
    })
}

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
                user: node_info.user().and_then(|r| r.ok()).map(|s| s.to_owned()),
                uid: node_info.uid(),
                timestamp: timestamp_conversion_wrapper(node_info.milli_timestamp()),
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
                    user: dense_node_info.user().map(|r| r.to_owned()).ok(),
                    uid: Some(dense_node_info.uid()),
                    timestamp: convert_timestamp(dense_node_info.milli_timestamp()).ok(),
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
                user: way_info.user().and_then(|r| r.ok()).map(|s| s.to_owned()),
                uid: way_info.uid(),
                timestamp: timestamp_conversion_wrapper(way_info.milli_timestamp()),
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
                user: relation_info
                    .user()
                    .and_then(|r| r.ok())
                    .map(|s| s.to_owned()),
                uid: relation_info.uid(),
                timestamp: timestamp_conversion_wrapper(relation_info.milli_timestamp()),
                visible: Some(relation_info.visible()),
                version: relation_info.version(),
            }
        }
    }
}

fn build_metadata_from_block(header_block: Box<HeaderBlock>) -> Metadata {
    Metadata {
        version: None,
        generator: header_block.writing_program().map(|s| s.to_owned()),
        copyright: None,
        license: None,
        timestamp: timestamp_conversion_wrapper(header_block.osmosis_replication_timestamp()),
    }
}

pub struct PbfReader {}

impl PbfReader {
    pub fn new() -> Self {
        PbfReader {}
    }
}

impl Reader for PbfReader {
    fn read_file(
        self,
        src: Option<PathBuf>,
        metadata_sender: Sender<Metadata>,
        _chunk_builder: ChunkBuilder,
    ) -> impl ParallelIterator<Item = ElementChunk> {
        let src = super::get_reader(src);
        let reader = BlobReader::new(src);

        reader
            .filter_map(move |blob| match blob.unwrap().decode() {
                Ok(BlobDecode::OsmData(block)) => Some(block),
                Ok(BlobDecode::OsmHeader(block)) => {
                    metadata_sender
                        .send(build_metadata_from_block(block))
                        .expect("Couldn't send metadata to main thread!");
                    None
                }
                Err(e) => panic!("ERROR: unable to read PBF input: {e:?}"),
                _ => None,
            })
            .enumerate()
            .par_bridge()
            .map(|(block_index, block)| ElementChunk {
                index: block_index,
                content: block
                    .elements()
                    .map(convert_element)
                    .collect::<Vec<Element>>()
                    .into_boxed_slice(),
            })
    }
}
