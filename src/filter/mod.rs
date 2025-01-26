//! Filters/transforms OSM data.

#[cfg(feature = "cel")]
mod cel;
#[cfg(feature = "cel")]
use cel::compile_cel_filter;

#[cfg(feature = "skyfilter")]
mod skyfilter;
#[cfg(feature = "skyfilter")]
use skyfilter::parse::parse_filter;

use std::{fmt::Error, fs::read_to_string, path::Path};

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

pub fn filter_from_path(value: &Path) -> Result<Box<dyn ElementFilter>, SkywayError> {
    match read_to_string(value) {
        Ok(contents) => match create_filter(&contents) {
            Ok(f) => Ok(f),
            Err(_) => Err(SkywayError::UnparsableFilter(
                value.to_str().unwrap().to_owned(), // TODO: clean this up
            )),
        },
        Err(_) => Err(SkywayError::InvalidFilterFile(
            value.to_str().unwrap().to_owned(), // TODO: clean this up
        )),
    }
}

fn create_filter(filter_contents: &str) -> Result<Box<dyn ElementFilter>, Error> {
    #[cfg(feature = "skyfilter")]
    if let Some(f) = parse_filter(filter_contents) {
        return Ok(Box::new(f));
    }

    #[cfg(feature = "cel")]
    if let Ok(f) = compile_cel_filter(filter_contents) {
        return Ok(Box::new(f));
    }

    Err(Error)
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
