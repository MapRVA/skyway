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

fn get_referenced_ids(
    id: &i64,
    relation_references: &HashMap<i64, Vec<i64>>,
    way_references: &HashMap<i64, Vec<i64>>,
) -> Vec<i64> {
    let mut referenced_ids = Vec::new();
    if let Some(relation_refs) = relation_references.get(&id) {
        for r in relation_refs {
            referenced_ids.extend(get_referenced_ids(r, relation_references, way_references));
        }
    } else if let Some(way_refs) = way_references.get(&id) {
        referenced_ids.extend(way_refs);
    } else {
        referenced_ids.push(*id);
    }
    referenced_ids
}

pub fn build_keep_list(
    filters: Vec<Box<dyn ElementFilter>>,
    chunk_receiver: Receiver<Chunk<Box<[Element]>>>,
) -> HashSet<i64> {
    let mut keep_ids = HashSet::new();

    let mut relation_references = HashMap::new();
    let mut way_references = HashMap::new();

    for mut chunk in chunk_receiver {
        for element in chunk.content.iter_mut() {
            match &element.element_type {
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
            for filter in &filters {
                if filter.evaluate(element) {
                    keep_ids.insert(element.id);
                }
            }
        }
    }

    let mut keep_ids_with_references = keep_ids.clone();
    for id in keep_ids {
        keep_ids_with_references.extend(get_referenced_ids(
            &id,
            &relation_references,
            &way_references,
        ))
    }
    keep_ids_with_references
}

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
