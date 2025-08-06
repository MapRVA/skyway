use std::collections::HashMap;

use geo::{
    Contains, Geometry, Intersects, LineString, MultiLineString, MultiPolygon, Polygon, Validation,
    coord,
};

use crate::{
    coord_to_f64,
    elements::{Element, ElementType},
};

use super::{TaggedGeometry, way_to_linestring};

// ----- MultiLineStrings -----

fn build_multilinestring(
    relation: &Element,
    all_elements: &HashMap<i64, Element>,
) -> Option<TaggedGeometry> {
    let ElementType::Relation { members } = &relation.element_type else {
        unreachable!();
    };

    let linestrings: Vec<LineString> = members
        .iter()
        .filter_map(|member| all_elements.get(&member.id))
        .filter_map(|element| match &element.element_type {
            ElementType::Way { nodes } => way_to_linestring(nodes, all_elements),
            _ => None,
        })
        .collect();

    // TODO: Merge connected linestrings

    match linestrings.len() {
        0 => None,
        _ => Some(TaggedGeometry {
            geometry: Geometry::MultiLineString(MultiLineString::new(linestrings)),
            tags: relation.tags.clone(),
        }),
    }
}

enum WayOrientation<'a> {
    Forward(&'a Element),
    Backward(&'a Element),
}

impl<'a> WayOrientation<'a> {
    fn nodes(&self) -> Vec<i64> {
        match self {
            WayOrientation::Forward(elem) => match &elem.element_type {
                ElementType::Way { nodes } => nodes.clone(),
                _ => Vec::new(),
            },
            WayOrientation::Backward(elem) => match &elem.element_type {
                ElementType::Way { nodes } => {
                    let mut reversed = nodes.clone();
                    reversed.reverse();
                    reversed
                }
                _ => Vec::new(),
            },
        }
    }
}

// ----- MultiPolygon Ring Assignment -----

fn ring_is_closed(ring: &[WayOrientation]) -> bool {
    if ring.is_empty() {
        return false;
    }

    let first_way = match ring.first().unwrap() {
        WayOrientation::Forward(elem) => elem,
        WayOrientation::Backward(elem) => elem,
    };

    let last_way = match ring.last().unwrap() {
        WayOrientation::Forward(elem) => elem,
        WayOrientation::Backward(elem) => elem,
    };

    match (&first_way.element_type, &last_way.element_type) {
        (ElementType::Way { nodes: first_nodes }, ElementType::Way { nodes: last_nodes }) => {
            let first_start = match ring.first().unwrap() {
                WayOrientation::Forward(_) => first_nodes.first(),
                WayOrientation::Backward(_) => first_nodes.last(),
            };

            let last_end = match ring.last().unwrap() {
                WayOrientation::Forward(_) => last_nodes.last(),
                WayOrientation::Backward(_) => last_nodes.first(),
            };

            first_start == last_end
        }
        _ => false,
    }
}

/// Take unassigned ways and attempt to form closed rings from them
fn ring_assignment<'a>(mut unassigned_ways: Vec<&'a Element>) -> Vec<Vec<WayOrientation<'a>>> {
    let mut rings = Vec::new();
    let mut current_ring: Vec<WayOrientation> = Vec::new();

    // start a new ring
    while let Some(way) = unassigned_ways.pop() {
        current_ring.push(WayOrientation::Forward(way));

        while !ring_is_closed(&current_ring) {
            let (_, current_last_node) = match current_ring.last().unwrap() {
                WayOrientation::Forward(elem) => match &elem.element_type {
                    ElementType::Way { nodes } => (elem, nodes.last()),
                    _ => unreachable!(),
                },
                WayOrientation::Backward(elem) => match &elem.element_type {
                    ElementType::Way { nodes } => (elem, nodes.first()),
                    _ => unreachable!(),
                },
            };

            let mut found_index = None;
            let mut found_orientation = None;

            for (i, remaining_way) in unassigned_ways.iter().enumerate() {
                let remaining_nodes = match &remaining_way.element_type {
                    ElementType::Way { nodes } => nodes,
                    _ => continue,
                };

                if remaining_nodes.first() == current_last_node {
                    found_index = Some(i);
                    found_orientation = Some(true); // forward
                    break;
                } else if remaining_nodes.last() == current_last_node {
                    found_index = Some(i);
                    found_orientation = Some(false); // backward
                    break;
                }
            }

            if let (Some(index), Some(is_forward)) = (found_index, found_orientation) {
                let way = unassigned_ways.remove(index);
                if is_forward {
                    current_ring.push(WayOrientation::Forward(way));
                } else {
                    current_ring.push(WayOrientation::Backward(way));
                }
            } else {
                // No matching way found - ring cannot be closed
                return Vec::new();
            }
        }
        rings.push(std::mem::take(&mut current_ring));
    }
    rings
}

// ----- MultiPolygon Ring Grouping -----

fn build_containment_matrix(ring_polygons: &[Option<Polygon>]) -> Vec<Vec<bool>> {
    let n = ring_polygons.len();
    let mut matrix = vec![vec![false; n]; n];

    for i in 0..n {
        for j in 0..n {
            if i != j {
                if let (Some(poly_i), Some(poly_j)) = (&ring_polygons[i], &ring_polygons[j]) {
                    matrix[i][j] = poly_i.contains(poly_j);
                }
            }
        }
    }
    matrix
}

fn find_outer_ring(used: &[bool], containment: &[Vec<bool>]) -> Option<usize> {
    (0..used.len())
        .find(|&i| !used[i] && !(0..used.len()).any(|j| j != i && !used[j] && containment[j][i]))
}

fn find_holes_for_outer(outer_idx: usize, used: &[bool], containment: &[Vec<bool>]) -> Vec<usize> {
    (0..used.len())
        .filter(|&i| {
            !used[i]
                && i != outer_idx
                && containment[outer_idx][i]
                && !(0..used.len())
                    .any(|j| j != i && j != outer_idx && !used[j] && containment[j][i])
        })
        .collect()
}

fn ring_to_linestring(ring: &[WayOrientation], all_elements: &HashMap<i64, Element>) -> LineString {
    let mut points = Vec::new();
    for way in ring {
        for node_id in way.nodes() {
            match all_elements.get(&node_id) {
                Some(e) => match &e.element_type {
                    ElementType::Node { lat, lon } => {
                        points.push(coord! {x: coord_to_f64(*lon), y: coord_to_f64(*lat)})
                    }
                    _ => unreachable!(),
                },
                None => unreachable!(),
            }
        }
    }
    LineString::new(points)
}

/// Find out which rings are nested into which other rings, and build polygons from them
fn ring_grouping<'a>(
    rings: Vec<Vec<WayOrientation<'a>>>,
    relation: &Element,
    all_elements: &HashMap<i64, Element>,
) -> Vec<TaggedGeometry> {
    if rings.is_empty() {
        return Vec::new();
    }

    let n = rings.len();
    // let mut additional_polygons = Vec::new();
    let mut used_rings = vec![false; n];
    let mut out_polygons = Vec::new();

    // Convert rings to polygons for geometric operations
    let ring_polygons: Vec<Option<Polygon>> = rings
        .iter()
        .map(|ring| ring_to_polygon(ring, all_elements))
        .collect();

    let containment_matrix = build_containment_matrix(&ring_polygons);

    while let Some(outer_idx) = find_outer_ring(&used_rings, &containment_matrix) {
        used_rings[outer_idx] = true;

        // RG-4 Final all unused rings contained by ring outer_idx, but not contained by other unused rings
        let hole_indices = find_holes_for_outer(outer_idx, &used_rings, &containment_matrix);

        // Mark hole rings as used
        for &hole_idx in &hole_indices {
            used_rings[hole_idx] = true;
        }

        // RG-5: Handle rings with different tags than the relation

        // additional_polygons.extend(
        //     hole_indices
        //         .iter()
        //         .filter(|&&idx| {
        //             rings[idx].iter().any(|orientation| {
        //                 let way = match orientation {
        //                     WayOrientation::Forward(elem) => elem,
        //                     WayOrientation::Backward(elem) => elem,
        //                 };
        //                 !way.tags.is_empty() && way.tags != relation.tags
        //             })
        //         })
        //         .map(|&idx| {
        //             let mut combined_tags = HashMap::new();
        //             for orientation in &rings[idx] {
        //                 let way = match orientation {
        //                     WayOrientation::Forward(elem) => elem,
        //                     WayOrientation::Backward(elem) => elem,
        //                 };
        //                 combined_tags.extend(way.tags.clone());
        //             }
        //             let polygon = ring_to_polygon(&rings[idx], all_elements);
        //             additional_polygons.push(TaggedGeometry {
        //                 geometry: polygon,
        //                 tags: combined_tags,
        //             });
        //         }),
        // );

        // RG-7: Construct polygon and test its validity
        let outer_linestring = ring_to_linestring(&rings[outer_idx], all_elements);
        let inner_linestrings: Vec<LineString> = hole_indices
            .into_iter()
            .map(|idx| ring_to_linestring(&rings[idx], all_elements))
            .collect();

        let polygon = Polygon::new(outer_linestring, inner_linestrings);

        if polygon.is_valid() {
            out_polygons.push(polygon);
        } else {
            return Vec::new();
        }
    }

    // Let's go ahead and take care of Multipolygon Creation
    let mut out_vec = Vec::new();
    match out_polygons.len() {
        0 => return Vec::new(),
        1 => {
            // MC-2
            out_vec.push(TaggedGeometry {
                geometry: Geometry::Polygon(out_polygons[0].clone()),
                tags: relation.tags.clone(),
            });
        }
        _ => {
            // MC-1
            for (i, polygon) in out_polygons[..out_polygons.len() - 1].iter().enumerate() {
                for other_polygon in out_polygons[i + 1..].iter() {
                    if polygon.intersects(other_polygon) {
                        return Vec::new();
                    }
                }
            }
            // MC-2
            out_vec.push(TaggedGeometry {
                geometry: Geometry::MultiPolygon(MultiPolygon::new(out_polygons)),
                tags: relation.tags.clone(),
            });
        }
    }

    // Add additional polygons from RG-5 to out_vec?

    out_vec
}

/// Convert a ring of ways into a Polygon
fn ring_to_polygon(
    ring: &Vec<WayOrientation>,
    all_elements: &HashMap<i64, Element>,
) -> Option<Polygon> {
    let mut all_points = Vec::new();

    for way in ring {
        for node_id in way.nodes() {
            let node_element = all_elements.get(&node_id)?;
            let ElementType::Node { lat, lon } = node_element.element_type else {
                return None; // Node ID references non-node element
            };
            all_points.push(coord! {x: coord_to_f64(lon), y: coord_to_f64(lat)});
        }
    }

    if all_points.len() < 3 {
        return None;
    }

    // Ensure the ring is closed
    if all_points.first() != all_points.last() {
        all_points.extend(all_points.first().copied());
    }

    let output_polygon = Polygon::new(LineString::new(all_points), Vec::new());
    output_polygon.is_valid().then_some(output_polygon)
}

/// Build multipolygon geometries from a relation
fn build_multipolygon(
    relation: &Element,
    all_elements: &HashMap<i64, Element>,
) -> Vec<TaggedGeometry> {
    let ElementType::Relation { members } = &relation.element_type else {
        unreachable!();
    };

    // RA-1: Collect all member ways into unassigned_ways Vec
    let unassigned_ways: Vec<&Element> = members
        .iter()
        .filter_map(|member| all_elements.get(&member.id))
        .filter(|element| matches!(&element.element_type, ElementType::Way { .. }))
        .collect();

    // Ring Assignment: Form closed rings from unassigned ways
    let rings = ring_assignment(unassigned_ways);

    if rings.is_empty() {
        return Vec::new();
    }

    // Ring Grouping: Determine ring relationships and group into polygons
    ring_grouping(rings, relation, all_elements)
}

pub fn construct_relation_geometry(
    relation: &Element,
    all_elements: &HashMap<i64, Element>,
) -> Vec<TaggedGeometry> {
    match relation.tags.get("type").map(|s| s.as_str()) {
        Some("multipolygon" | "boundary") => build_multipolygon(relation, all_elements),
        Some("multilinestring") => {
            build_multilinestring(relation, all_elements).map_or_else(|| Vec::new(), |g| vec![g])
        }
        _ => Vec::new(),
    }
}
