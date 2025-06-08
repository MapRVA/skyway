//! Filters/transforms OSM data.

#[cfg(feature = "cel")]
mod cel;
#[cfg(feature = "cel")]
use cel::compile_cel_filter;

#[cfg(feature = "skyfilter")]
mod skyfilter;
#[cfg(feature = "skyfilter")]
use skyfilter::parse::parse_filter;

use std::{
    collections::{HashMap, HashSet},
    fs::read_to_string,
    path::Path,
    sync::mpsc::Receiver,
};

use crate::{
    SkywayError,
    chunks::{Chunk, ElementChunk},
    elements::{Element, ElementType},
};

/// Represents a filter that can be evaluated on an `Element`, transforming it.
pub trait ElementFilter: Send + Sync {
    fn evaluate(&self, element: &mut Element) -> bool;
}

enum Filter {
    SkyFilter,
    Cel,
}

fn auto_parse_filter(filter_contents: &str) -> Result<Box<dyn ElementFilter>, SkywayError> {
    #[cfg(feature = "skyfilter")]
    if let Ok(f) = parse_filter(filter_contents) {
        return Ok(Box::new(f));
    }

    #[cfg(feature = "cel")]
    if let Ok(f) = compile_cel_filter(filter_contents) {
        return Ok(Box::new(f));
    }

    Err(SkywayError::UnparsableFilter("Unable to parse filter. Please use a recognized file extension to get a more helpful parsing error message.".to_string()))
}

pub fn filter_from_path(filter_path: &Path) -> Result<Box<dyn ElementFilter>, SkywayError> {
    // do we recognize the filter type based on its path?
    let filter_type: Option<Filter> = match filter_path.extension() {
        Some(e) => match e.to_str() {
            Some("skyfilter") => Some(Filter::SkyFilter),
            Some("cel") => Some(Filter::Cel),
            _ => None,
        },
        None => None,
    };

    match read_to_string(filter_path) {
        Ok(contents) => match filter_type {
            Some(Filter::SkyFilter) => Ok(Box::new(parse_filter(&contents)?)),
            Some(Filter::Cel) => Ok(Box::new(compile_cel_filter(&contents)?)),
            None => auto_parse_filter(&contents),
        },
        Err(_) => Err(SkywayError::InvalidFilterFile(match filter_path.to_str() {
            Some(s) => format!("Unable to read filter file {}, does it exist?", s),
            None => "Unable to parse file path or its contents as string.".to_string(),
        })),
    }
}

/// Given a list of all way and relation references in a dataset,
/// return all elements referenced by element with given id, recursive.
fn get_referenced_ids(
    id: &i64,
    relation_references: &HashMap<i64, Vec<i64>>,
    way_references: &HashMap<i64, Vec<i64>>,
) -> Option<Vec<i64>> {
    if let Some(relation_refs) = relation_references.get(&id) {
        // Element is relation, we must recursively resolve its references.
        // (Because relations can reference other relations!)
        let mut recursive_referenced_ids = Vec::new();
        for r in relation_refs {
            if let Some(referenced_ids) = get_referenced_ids(r, relation_references, way_references)
            {
                recursive_referenced_ids.extend(referenced_ids);
            }
        }
        Some(recursive_referenced_ids)
    } else if let Some(way_refs) = way_references.get(&id) {
        // Element is way, we can quickly determine its references.
        Some(way_refs.to_owned())
    } else {
        // Element is node, no potential references.
        None
    }
}

pub fn build_keep_list(
    filters: &Vec<Box<dyn ElementFilter>>,
    chunk_receiver: Receiver<Chunk<Box<[Element]>>>,
) -> HashSet<i64> {
    let mut keep_ids = HashSet::new();

    // HashMap that stores every relation ID, along with
    // every element ID that it references.
    let mut relation_references: HashMap<i64, Vec<i64>> = HashMap::new();

    // HashMap that stores every way ID, along with every
    // node ID that it references.
    let mut way_references: HashMap<i64, Vec<i64>> = HashMap::new();

    // Iterate over every input element, storing all
    // references in the above two HashMaps.
    for mut chunk in chunk_receiver {
        for element in chunk.content.iter_mut() {
            match &element.element_type {
                // Ignore nodes, they don't reference anything!
                //
                // Note that nodes may still be kept by filters below.
                ElementType::Node { .. } => (),
                ElementType::Way { nodes } => {
                    // FIXME: return some kind of error if this element already exists in the HashMap
                    way_references.insert(element.id, nodes.clone());
                }
                ElementType::Relation { members } => {
                    let mut member_ids = Vec::new();
                    for member in members {
                        member_ids.push(member.id);
                    }
                    relation_references.insert(element.id, member_ids);
                }
            }

            // Keep elements that pass the filters.
            for filter in filters {
                if filter.evaluate(element) {
                    keep_ids.insert(element.id);
                }
            }
        }
    }

    // Now we need to add the IDs of all elements referenced
    // by kept elements to keep_ids, recursively.
    let mut keep_ids_with_references = keep_ids.clone();
    for id in keep_ids {
        if let Some(referenced_ids) = get_referenced_ids(&id, &relation_references, &way_references)
        {
            keep_ids_with_references.extend(referenced_ids)
        }
    }

    // Return all IDs of kept elements + their references.
    keep_ids_with_references
}

/// Transform a Vec of boxed ElementFilters into a single function that filters an ElementChunk
pub fn build_filter(
    filters: Vec<Box<dyn ElementFilter>>,
) -> Box<dyn Fn(ElementChunk) -> ElementChunk + Sync> {
    Box::new(move |chunk: ElementChunk| {
        let mut out_elements = Vec::new();
        for mut element in chunk.content.into_iter() {
            let mut keep_element = true;
            for filter in &filters {
                if !filter.evaluate(&mut element) {
                    keep_element = false;
                    break;
                }
            }
            if keep_element {
                out_elements.push(element);
            }
        }
        Chunk {
            content: out_elements.into_boxed_slice(),
            index: chunk.index,
        }
    })
}

#[cfg(test)]
mod tests {

    use super::*;
    const VERSION: &str = env!("CARGO_PKG_VERSION");

    #[test]
    fn test_auto_parse_skyfilter() {
        let filter_contents = format!("SkyFilter v{}\n\nCOMMIT", VERSION);
        let parsed_filter = auto_parse_filter(&filter_contents);
        assert!(parsed_filter.is_ok());
    }

    #[test]
    fn test_auto_parse_cel() {
        let filter_contents = format!("type == \"way\"");
        let parsed_filter = auto_parse_filter(&filter_contents);
        assert!(parsed_filter.is_ok());
    }

    #[test]
    fn test_auto_parse_error() {
        let filter_contents = format!("notafilter!!!");
        let parsed_filter = auto_parse_filter(&filter_contents);
        assert!(parsed_filter.is_err());
    }
}
