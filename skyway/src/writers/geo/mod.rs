use std::{collections::HashMap, sync::mpsc::Receiver};

use geo::{Geometry, Point, Polygon};

use crate::{
    SkywayError,
    chunks::ElementChunk,
    coord_to_f64,
    elements::{Element, ElementKey, ElementType},
};

mod ways;
pub use ways::{way_is_area, way_to_linestring};

mod relations;
use relations::construct_relation_geometry;

pub struct TaggedGeometry {
    pub geometry: Geometry,
    pub tags: HashMap<String, String>,
}

fn elements_to_geometry_collection(
    all_elements: HashMap<ElementKey, Element>,
) -> Result<Vec<TaggedGeometry>, SkywayError> {
    let mut geometries = Vec::new();

    for element in all_elements.values() {
        if !element.tags.is_empty() {
            match &element.element_type {
                ElementType::Node { lat, lon } => {
                    geometries.push(TaggedGeometry {
                        geometry: Geometry::Point(Point::new(
                            coord_to_f64(*lon),
                            coord_to_f64(*lat),
                        )),
                        tags: element.tags.clone(),
                    });
                }
                ElementType::Way { nodes } => {
                    if let Some(way_linestring) = way_to_linestring(nodes, &all_elements) {
                        if way_is_area(&way_linestring, &element.tags) {
                            geometries.push(TaggedGeometry {
                                geometry: Geometry::Polygon(Polygon::new(
                                    way_linestring,
                                    Vec::new(),
                                )),
                                tags: element.tags.clone(),
                            });
                        } else {
                            geometries.push(TaggedGeometry {
                                geometry: Geometry::LineString(way_linestring),
                                tags: element.tags.clone(),
                            });
                        }
                    }
                }
                ElementType::Relation { .. } => {
                    geometries.extend(construct_relation_geometry(element, &all_elements));
                }
            }
        }
    }

    return Ok(geometries);
}

pub fn convert_chunks(
    element_receiver: Receiver<ElementChunk>,
) -> Result<Vec<TaggedGeometry>, SkywayError> {
    let mut all_elements: HashMap<ElementKey, Element> = HashMap::new();

    for chunk in element_receiver {
        for element in chunk {
            all_elements.insert(element.key(), element);
        }
    }
    elements_to_geometry_collection(all_elements)
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::channel;

    use crate::elements::{Member, SimpleElementType};

    use super::*;

    fn element(element_type: ElementType, id: i64, tags: &[(&str, &str)]) -> Element {
        Element {
            changeset: None,
            user: None,
            version: None,
            uid: None,
            id,
            timestamp: None,
            visible: None,
            tags: tags
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<HashMap<_, _>>(),
            element_type,
        }
    }

    fn node(id: i64, lat: i32, lon: i32, tags: &[(&str, &str)]) -> Element {
        element(ElementType::Node { lat, lon }, id, tags)
    }

    fn way(id: i64, nodes: &[i64], tags: &[(&str, &str)]) -> Element {
        element(
            ElementType::Way {
                nodes: nodes.to_vec(),
            },
            id,
            tags,
        )
    }

    fn relation(id: i64, members: Vec<Member>, tags: &[(&str, &str)]) -> Element {
        element(ElementType::Relation { members }, id, tags)
    }

    /// Run `convert_chunks` over a single chunk of elements.
    fn convert(elements: Vec<Element>) -> Vec<TaggedGeometry> {
        let (sender, receiver) = channel();
        sender
            .send(ElementChunk {
                index: 0,
                content: elements.into_boxed_slice(),
            })
            .unwrap();
        drop(sender);

        convert_chunks(receiver).unwrap()
    }

    #[test]
    fn elements_of_different_types_sharing_an_id_both_produce_geometry() {
        let geometries = convert(vec![
            node(1, 0, 0, &[]),
            node(2, 10_000_000, 0, &[]),
            node(7, 20_000_000, 30_000_000, &[("amenity", "cafe")]),
            way(7, &[1, 2], &[("highway", "residential")]),
        ]);

        assert_eq!(geometries.len(), 2, "neither element overwrote the other");
        assert!(
            geometries
                .iter()
                .any(|g| matches!(g.geometry, Geometry::Point(_)))
        );
        assert!(
            geometries
                .iter()
                .any(|g| matches!(g.geometry, Geometry::LineString(_)))
        );
    }

    #[test]
    fn relation_members_resolve_by_type_not_by_id_alone() {
        let geometries = convert(vec![
            node(1, 0, 0, &[]),
            node(2, 10_000_000, 0, &[]),
            way(7, &[1, 2], &[]),
            // Same ID as the way above, so an untyped lookup would find this
            // node instead, and the relation would lose its only member.
            node(7, 20_000_000, 30_000_000, &[]),
            relation(
                3,
                vec![Member {
                    t: Some(SimpleElementType::Way),
                    id: 7,
                    role: None,
                }],
                &[("type", "multilinestring")],
            ),
        ]);

        assert_eq!(geometries.len(), 1);
        assert!(matches!(
            geometries[0].geometry,
            Geometry::MultiLineString(_)
        ));
    }

    #[test]
    fn untyped_relation_members_are_skipped() {
        let geometries = convert(vec![
            node(1, 0, 0, &[]),
            node(2, 10_000_000, 0, &[]),
            way(7, &[1, 2], &[]),
            relation(
                3,
                vec![Member {
                    t: None,
                    id: 7,
                    role: None,
                }],
                &[("type", "multilinestring")],
            ),
        ]);

        assert!(
            geometries.is_empty(),
            "a member with no type cannot be resolved to an element"
        );
    }
}
