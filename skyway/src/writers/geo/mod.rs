use std::{collections::HashMap, sync::mpsc::Receiver};

use geo::{Geometry, Point, Polygon};

use crate::{
    SkywayError,
    chunks::ElementChunk,
    coord_to_f64,
    elements::{Element, ElementType},
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
    all_elements: HashMap<i64, Element>,
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
    let mut all_elements: HashMap<i64, Element> = HashMap::new();

    for chunk in element_receiver {
        for element in chunk {
            all_elements.insert(element.id, element);
        }
    }
    elements_to_geometry_collection(all_elements)
}
