//! Filters/transforms OSM data.

#[cfg(feature = "cel")]
mod cel;
#[cfg(feature = "cel")]
use cel::compile_cel_filter;

#[cfg(feature = "skyfilter")]
mod skyfilter;
#[cfg(feature = "skyfilter")]
use skyfilter::parse::parse_filter;

use std::{fs::read_to_string, path::Path};

use crate::{chunks::ElementChunk, elements::Element, SkywayError};

/// Represents a filter that can be evaluated on an `Element`, transforming it.
pub trait ElementFilter: Send + Sync {
    fn evaluate(&self, element: &mut Element) -> bool;

    fn evaluate_option(&self, mut element: Element) -> Option<Element> {
        match self.evaluate(&mut element) {
            true => Some(element),
            false => None,
        }
    }
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

pub fn build_filter(
    filters: Vec<Box<dyn ElementFilter>>,
) -> Box<dyn Fn(ElementChunk) -> ElementChunk + Sync> {
    Box::new(move |mut chunk: ElementChunk| {
        for filter in &filters {
            chunk = ElementChunk {
                index: chunk.index,
                content: chunk
                    .content
                    .into_vec()
                    .into_iter()
                    .filter_map(|element| filter.evaluate_option(element))
                    .collect(),
            }
        }
        chunk
    })
}
