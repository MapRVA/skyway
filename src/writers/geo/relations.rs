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

// ----- MultiPolygon Helper Structs -----

struct PolygonGroup<'a> {
    outer: Vec<&'a Element>,
    holes: Vec<Vec<&'a Element>>,
}

struct RingGroupingResult<'a> {
    groups: Vec<PolygonGroup<'a>>,
    additional: Vec<AdditionalPolygon<'a>>,
}

struct AdditionalPolygon<'a> {
    ring: Vec<&'a Element>,
    tags: HashMap<String, String>,
}

// ----- MultiPolygon Ring Assignment -----

fn ring_is_closed(ring: &[&Element]) -> bool {
    if ring.is_empty() {
        return false;
    }

    match (&ring[0].element_type, &ring.last().unwrap().element_type) {
        (ElementType::Way { nodes: first }, ElementType::Way { nodes: last }) => {
            first.first() == last.last()
        }
        _ => false,
    }
}

/// Take unassigned ways and attempt to form closed rings from them
fn ring_assignment<'a>(
    mut unassigned_ways: Vec<(&'a i64, &'a ElementType)>,
    all_elements: &'a HashMap<i64, Element>,
) -> Vec<Vec<&'a Element>> {
    let mut rings = Vec::new();
    let mut current_ring: Vec<&Element> = Vec::new();

    while let Some((way_id, _)) = unassigned_ways.pop().or_else(|| {
        if !current_ring.is_empty() {
            unassigned_ways.last().map(|&x| x)
        } else {
            None
        }
    }) {
        current_ring.push(&all_elements[&way_id]);

        if ring_is_closed(&current_ring) {
            rings.push(std::mem::take(&mut current_ring));
            continue;
        }

        // RA-4: If the current ring is not closed, get the end node of the current ring
        let Some(end_node) = current_ring
            .last()
            .and_then(|elem| match &elem.element_type {
                ElementType::Way { nodes } => nodes.last(),
                _ => None,
            })
        else {
            break;
        };

        if let Some(idx) = unassigned_ways.iter().position(|(_, way_type)| {
            matches!(way_type, ElementType::Way { nodes } if
                nodes.first() == Some(end_node) || nodes.last() == Some(end_node))
        }) {
            let (connecting_id, _) = unassigned_ways.remove(idx);
            current_ring.push(&all_elements[&connecting_id]);
        } else {
            break;
        }
    }

    // There could be a dangling ring!
    if !current_ring.is_empty() && ring_is_closed(&current_ring) {
        rings.push(current_ring);
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

/// Find out which rings are nested into which other rings, and build polygons from them
fn ring_grouping<'a>(
    rings: Vec<Vec<&'a Element>>,
    relation: &Element,
    all_elements: &HashMap<i64, Element>,
) -> Option<RingGroupingResult<'a>> {
    if rings.is_empty() {
        return Some(RingGroupingResult {
            groups: Vec::new(),
            additional: Vec::new(),
        });
    }

    let n = rings.len();
    let mut polygon_groups = Vec::new();
    let mut additional_polygons = Vec::new();
    let mut used_rings = vec![false; n];

    // Convert rings to polygons for geometric operations
    let ring_polygons: Vec<Option<Polygon>> = rings
        .iter()
        .map(|ring| ring_to_polygon(ring, all_elements))
        .collect::<Option<Vec<_>>>()?
        .into_iter()
        .map(Some)
        .collect();

    let containment_matrix = build_containment_matrix(&ring_polygons);

    while let Some(outer_idx) = find_outer_ring(&used_rings, &containment_matrix) {
        used_rings[outer_idx] = true;

        let hole_indices = find_holes_for_outer(outer_idx, &used_rings, &containment_matrix);

        // Mark hole rings as used
        for &hole_idx in &hole_indices {
            used_rings[hole_idx] = true;
        }

        // RG-5: Handle rings with different tags than the relation
        additional_polygons.extend(
            hole_indices
                .iter()
                .filter(|&&idx| {
                    rings[idx]
                        .iter()
                        .any(|way| !way.tags.is_empty() && way.tags != relation.tags)
                })
                .map(|&idx| {
                    let mut combined_tags = HashMap::new();
                    for way in &rings[idx] {
                        combined_tags.extend(way.tags.clone());
                    }
                    AdditionalPolygon {
                        ring: rings[idx].clone(),
                        tags: combined_tags,
                    }
                }),
        );

        // RG-7: Construct polygon group
        let holes: Vec<Vec<&Element>> = hole_indices
            .into_iter()
            .map(|idx| rings[idx].clone())
            .collect();

        polygon_groups.push(PolygonGroup {
            outer: rings[outer_idx].clone(),
            holes,
        });
    }

    Some(RingGroupingResult {
        groups: polygon_groups,
        additional: additional_polygons,
    })
}

// ----- Multipolygon Creation -----

fn multipolygon_creation<'a>(
    polygon_groups: Vec<PolygonGroup<'a>>,
    all_elements: &HashMap<i64, Element>,
) -> Option<Geometry> {
    if polygon_groups.is_empty() {
        return None;
    }

    let polygons: Vec<Polygon> = polygon_groups
        .into_iter()
        .map(|group| {
            let outer = ring_to_polygon(&group.outer, all_elements)?
                .exterior()
                .clone();

            let holes: Vec<_> = group
                .holes
                .iter()
                .map(|ring| ring_to_polygon(ring, all_elements).map(|p| p.exterior().clone()))
                .collect::<Option<Vec<_>>>()?;

            Some(Polygon::new(outer, holes))
        })
        .collect::<Option<Vec<_>>>()?;

    // MC-1: Check for intersections between polygons
    for i in 0..polygons.len() {
        for j in (i + 1)..polygons.len() {
            if polygons[i].intersects(&polygons[j]) {
                return None;
            }
        }
    }

    // MC-2: Construct multipolygon from all polygons
    match polygons.len() {
        1 => Some(Geometry::Polygon(polygons.into_iter().next().unwrap())),
        _ => Some(Geometry::MultiPolygon(MultiPolygon::new(polygons))),
    }
}

/// Convert a ring of ways into a Polygon
fn ring_to_polygon(ring: &[&Element], all_elements: &HashMap<i64, Element>) -> Option<Polygon> {
    let mut all_points = Vec::new();

    for way in ring {
        let ElementType::Way { nodes } = &way.element_type else {
            return None; // Ring contains non-way element
        };

        for &node_id in nodes {
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
) -> Option<Vec<TaggedGeometry>> {
    let ElementType::Relation { members } = &relation.element_type else {
        unreachable!();
    };

    // RA-1: Collect all member ways into unassigned_ways Vec
    let unassigned_ways: Vec<(&i64, &ElementType)> = members
        .iter()
        .filter_map(|member| all_elements.get(&member.id))
        .filter_map(|element| match &element.element_type {
            w @ ElementType::Way { .. } => Some((&element.id, w)),
            _ => None,
        })
        .collect();

    // Ring Assignment: Form closed rings from unassigned ways
    let rings = ring_assignment(unassigned_ways, all_elements);

    if rings.is_empty() {
        return None;
    }

    // Ring Grouping: Determine ring relationships and group into polygons
    let RingGroupingResult { groups, additional } = ring_grouping(rings, relation, all_elements)?;

    // (Multi)polygon Creation: Convert to geo geometries
    let main_geometry = multipolygon_creation(groups, all_elements)?;

    let mut output_geometries = vec![TaggedGeometry {
        geometry: main_geometry,
        tags: relation.tags.clone(),
    }];

    // Add additional polygons from RG-5 (rings with different tags)
    output_geometries.extend(additional.iter().filter_map(|add| {
        ring_to_polygon(&add.ring, all_elements).map(|polygon| TaggedGeometry {
            geometry: Geometry::Polygon(polygon),
            tags: add.tags.clone(),
        })
    }));

    Some(output_geometries)
}

pub fn construct_relation_geometry(
    relation: &Element,
    all_elements: &HashMap<i64, Element>,
) -> Option<Vec<TaggedGeometry>> {
    match relation.tags.get("type").map(|s| s.as_str()) {
        Some("multipolygon" | "boundary") => build_multipolygon(relation, all_elements),
        Some("multilinestring") => build_multilinestring(relation, all_elements).map(|g| vec![g]),
        _ => None,
    }
}
