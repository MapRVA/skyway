use geo_types::{Geometry, LineString, MultiPolygon, Point, Polygon, coord};

use std::{collections::HashMap, sync::mpsc::Receiver};

use crate::{
    SkywayError,
    chunks::ElementChunk,
    coord_to_f64,
    elements::{Element, ElementType},
};

pub struct TaggedGeometry {
    pub geometry: Geometry,
    pub tags: HashMap<String, String>,
}

fn way_is_area(linestring: &LineString, tags: &HashMap<String, String>) -> bool {
    // Not an area if it does not form a closed loop
    //
    // This decision is based off the start and end
    // literally being the same node, rather than having the
    // same lat and lon values. I think that's better, but
    // it's worth noting.
    if !linestring.is_closed() {
        return false;
    }

    // Not an area if it is tagged `area=no`
    if tags.get("area").is_some_and(|s| s.eq("no")) {
        return false;
    }
    // Now, it could be an area! The following cases return true.
    // I have intentionally written these out as individual if
    // statements for readability and maintainability.
    {
        // Any other area tag
        if tags.contains_key("area") {
            return true;
        }

        // There is an `area:highway=*` tag and it is not "no"
        if tags.get("area:highway").is_some_and(|s| s.ne("no")) {
            return true;
        }

        // There is an `aeroway=*` tag and its value is not "no" or "taxiway"
        if tags
            .get("aeroway")
            .is_some_and(|s| s.ne("no") && s.ne("taxiway"))
        {
            return true;
        }

        // There is an `amenity=*` tag and its value is not "no"
        if tags.get("amenity").is_some_and(|s| s.ne("no")) {
            return true;
        }

        // There is a `barrier=*` tag and its value is one of:
        let barrier_area_values = vec![
            "city_wall",
            "ditch",
            "hedge",
            "retaining_wall",
            "wall",
            "spikes",
        ];
        if tags
            .get("barrier")
            .is_some_and(|s| barrier_area_values.iter().any(|&val| s == val))
        {
            return true;
        }

        // There is a `boundary=*` tag and its value is not "no"
        if tags.get("boundary").is_some_and(|s| s.ne("no")) {
            return true;
        }

        // There is a `building:part=*` tag and its value is not "no"
        if tags.get("building:part").is_some_and(|s| s.ne("no")) {
            return true;
        }

        // There is a `building=*` tag and its value is not "no"
        if tags.get("building").is_some_and(|s| s.ne("no")) {
            return true;
        }

        // There is a `craft=*` tag and its value is not "no"
        if tags.get("craft").is_some_and(|s| s.ne("no")) {
            return true;
        }

        // There is a `golf=*` tag and its value is not "no"
        if tags.get("golf").is_some_and(|s| s.ne("no")) {
            return true;
        }

        // There is a `highway=*` tag and its value is one of:
        let highway_area_values = vec!["services", "rest_area", "escape", "elevator"];
        if tags
            .get("highway")
            .is_some_and(|s| highway_area_values.iter().any(|&val| s == val))
        {
            return true;
        }

        // There is a `historic=*` tag and its value is not "no"
        if tags.get("historic").is_some_and(|s| s.ne("no")) {
            return true;
        }

        // There is a `indoor=*` tag and its value is not "no"
        if tags.get("indoor").is_some_and(|s| s.ne("no")) {
            return true;
        }

        // There is a `landuse=*` tag and its value is not "no"
        if tags.get("landuse").is_some_and(|s| s.ne("no")) {
            return true;
        }

        // There is a `leisure=*` tag and its value is not "no"
        if tags.get("leisure").is_some_and(|s| s.ne("no")) {
            return true;
        }

        // There is a `man_made=*` tag and its value is not one of:
        let man_made_not_area_values = vec!["no", "cutline", "embankment", "pipeline"];
        if tags
            .get("man_made")
            .is_some_and(|s| !man_made_not_area_values.iter().any(|&val| s == val))
        {
            return true;
        }

        // There is a `military=*` tag and its value is not "no"
        if tags.get("military").is_some_and(|s| s.ne("no")) {
            return true;
        }

        // There is a `natural=*` tag and its value is not one of:
        let natural_not_area_values =
            vec!["no", "coastline", "cliff", "ridge", "arete", "tree_row"];
        if tags
            .get("natural")
            .is_some_and(|s| !natural_not_area_values.iter().any(|&val| s == val))
        {
            return true;
        }

        // There is a `office=*` tag and its value is not "no"
        if tags.get("office").is_some_and(|s| s.ne("no")) {
            return true;
        }

        // There is a `place=*` tag and its value is not "no"
        if tags.get("place").is_some_and(|s| s.ne("no")) {
            return true;
        }

        // There is a `power=*` tag and its value is one of:
        let power_area_values = vec!["plant", "substation", "generator", "transformer"];
        if tags
            .get("power")
            .is_some_and(|s| power_area_values.iter().any(|&val| s == val))
        {
            return true;
        }

        // There is a `public_transport=*` tag and its value is not "no"
        if tags.get("public_transport").is_some_and(|s| s.ne("no")) {
            return true;
        }

        // There is a `railway=*` tag and its value is one of:
        let railway_area_values = vec!["station", "turntable", "roundhouse", "platform"];
        if tags
            .get("railway")
            .is_some_and(|s| railway_area_values.iter().any(|&val| s == val))
        {
            return true;
        }

        // There is a `ruins=*` tag and its value is not "no"
        if tags.get("ruins").is_some_and(|s| s.ne("no")) {
            return true;
        }

        // There is a `shop=*` tag and its value is not "no"
        if tags.get("shop").is_some_and(|s| s.ne("no")) {
            return true;
        }

        // There is a `tourism=*` tag and its value is not "no"
        if tags.get("tourism").is_some_and(|s| s.ne("no")) {
            return true;
        }

        // There is a `waterway=*` tag and its value is one of:
        let waterway_area_values = vec!["riverbank", "dock", "boatyard", "dam"];
        if tags
            .get("waterway")
            .is_some_and(|s| waterway_area_values.iter().any(|&val| s == val))
        {
            return true;
        }
    }

    // Truthy cases exhausted, so we default back to false.
    false
}

fn elements_to_geometry_collection(
    all_elements: HashMap<i64, Element>,
) -> Result<Vec<TaggedGeometry>, SkywayError> {
    let mut geometries = Vec::new();

    for element in all_elements.values() {
        match &element.element_type {
            ElementType::Node { lat, lon } => {
                if !element.tags.is_empty() {
                    geometries.push(TaggedGeometry {
                        geometry: Geometry::Point(Point::new(
                            coord_to_f64(*lon),
                            coord_to_f64(*lat),
                        )),
                        tags: element.tags.clone(),
                    });
                }
            }
            ElementType::Way { nodes } => {
                let mut points = Vec::new();
                for node_id in nodes {
                    match all_elements.get(&node_id) {
                        Some(e) => match e.element_type {
                            ElementType::Node { lat, lon } => {
                                points.push(coord! {x: coord_to_f64(lon), y: coord_to_f64(lat)})
                            }
                            _ => {
                                return Err(SkywayError::UnexpectedError(
                                    "Way unexpectedly references non-node element".to_string(),
                                ));
                            }
                        },
                        None => {
                            return Err(SkywayError::InvalidInputFile(format!(
                                "Unable to find node with ID {} to construct way {}",
                                node_id, element.id
                            )));
                        }
                    }
                }

                let way_linestring = LineString::new(points);

                if way_is_area(&way_linestring, &element.tags) {
                    geometries.push(TaggedGeometry {
                        geometry: Geometry::Polygon(Polygon::new(way_linestring, Vec::new())),
                        tags: element.tags.clone(),
                    });
                } else {
                    geometries.push(TaggedGeometry {
                        geometry: Geometry::LineString(way_linestring),
                        tags: element.tags.clone(),
                    });
                }
            }
            ElementType::Relation { members } => {
                unimplemented!()
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
