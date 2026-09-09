//! Filters/transforms OSM data.

#[cfg(feature = "cel")]
mod cel;
#[cfg(feature = "cel")]
use cel::compile_cel_filter;

#[cfg(feature = "skyfilter")]
mod skyfilter;
#[cfg(feature = "skyfilter")]
use skyfilter::parse::parse_filter;

mod references;
pub use references::{Discovery, discover_chunk, emit_chunk, filter_chunk, reference_closure};

use std::{fs::read_to_string, path::Path};

use crate::{SkywayError, elements::Element, plan::TransformFacts};

/// Capabilities a filter exposes to the pipeline planner.
///
/// Reference-preserving filtering evaluates every element twice: once to
/// discover which elements are selected and what they reference, and once
/// more to emit them. A filter is replay-safe when both evaluations produce
/// the same selection and transformed element. Preserving order means it
/// never changes the element type or ID used as an ordering key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FilterCapabilities {
    /// The filter can be evaluated again during reference-preserving emission.
    /// This includes repeatable behavior and stable identity/reference
    /// semantics across both evaluations.
    pub replay_safe: bool,
    /// The filter leaves the element type and ID ordering keys unchanged.
    pub preserves_order: bool,
}

impl FilterCapabilities {
    /// Assume nothing. Filters with unknown capabilities cannot be used with
    /// reference preservation.
    pub const UNKNOWN: Self = FilterCapabilities {
        replay_safe: false,
        preserves_order: false,
    };

    /// A replay-safe filter that only selects elements and edits tags.
    pub const TAGS_ONLY: Self = FilterCapabilities {
        replay_safe: true,
        preserves_order: true,
    };
}

/// Represents a filter that can be evaluated on an `Element`, transforming it.
pub trait ElementFilter: Send + Sync {
    /// Transform `element` in place. Returns whether it should be kept.
    fn evaluate(&self, element: &mut Element) -> bool;

    /// Capabilities this filter guarantees. The default is
    /// conservative; implementors should override it when they can promise
    /// more, otherwise reference-preserving filtering is refused.
    fn capabilities(&self) -> FilterCapabilities {
        FilterCapabilities::UNKNOWN
    }
}

/// Summarize what a set of filters may do to elements, for planning.
pub fn transform_facts(filters: &[Box<dyn ElementFilter>]) -> TransformFacts {
    let capabilities = filters.iter().fold(
        FilterCapabilities {
            replay_safe: true,
            preserves_order: true,
        },
        |combined, filter| {
            let filter = filter.capabilities();
            FilterCapabilities {
                replay_safe: combined.replay_safe && filter.replay_safe,
                preserves_order: combined.preserves_order && filter.preserves_order,
            }
        },
    );

    TransformFacts {
        has_filters: !filters.is_empty(),
        replay_safe: capabilities.replay_safe,
        preserves_order: capabilities.preserves_order,
    }
}

/// A compiled set of filters that can be evaluated as one unit.
///
/// For replay-based reference preservation the program must be replay-safe
/// and must leave `element` as the candidate that would be emitted if the
/// element is retained.
pub trait FilterProgram: Send + Sync {
    /// Transform `element` in place. Returns whether the element is selected
    /// directly by the filters.
    fn evaluate(&self, element: &mut Element) -> bool;
}

/// The configured filters, applied in order. An element is selected only if
/// every filter keeps it; evaluation stops at the first filter that drops it.
pub struct CompiledFilters {
    filters: Vec<Box<dyn ElementFilter>>,
}

impl CompiledFilters {
    pub fn new(filters: Vec<Box<dyn ElementFilter>>) -> Self {
        CompiledFilters { filters }
    }
}

impl FilterProgram for CompiledFilters {
    fn evaluate(&self, element: &mut Element) -> bool {
        self.filters.iter().all(|filter| filter.evaluate(element))
    }
}

pub fn auto_parse_filter(filter_contents: &str) -> Result<Box<dyn ElementFilter>, SkywayError> {
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
    match read_to_string(filter_path) {
        // do we recognize the filter type based on its path? an extension we
        // do not recognize, or one whose filter type was not compiled in,
        // falls back to sniffing the contents
        Ok(contents) => match filter_path.extension().and_then(|e| e.to_str()) {
            #[cfg(feature = "skyfilter")]
            Some("skyfilter") => Ok(Box::new(parse_filter(&contents)?)),
            #[cfg(feature = "cel")]
            Some("cel") => Ok(Box::new(compile_cel_filter(&contents)?)),
            _ => auto_parse_filter(&contents),
        },
        Err(_) => Err(SkywayError::InvalidFilterFile(match filter_path.to_str() {
            Some(s) => format!("Unable to read filter file {}, does it exist?", s),
            None => "Unable to parse file path or its contents as string.".to_string(),
        })),
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    #[cfg(feature = "skyfilter")]
    const VERSION: &str = env!("CARGO_PKG_VERSION");

    #[cfg(feature = "skyfilter")]
    #[test]
    fn test_auto_parse_skyfilter() {
        let filter_contents = format!("SkyFilter v{}\n\nCOMMIT", VERSION);
        let parsed_filter = auto_parse_filter(&filter_contents);
        assert!(parsed_filter.is_ok());
    }

    #[cfg(feature = "cel")]
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

    struct Unknown;

    impl ElementFilter for Unknown {
        fn evaluate(&self, _element: &mut Element) -> bool {
            true
        }
    }

    #[test]
    fn transform_facts_are_conservative_for_unknown_filters() {
        let facts = transform_facts(&[]);
        assert_eq!(facts, TransformFacts::none());

        let facts = transform_facts(&[Box::new(Unknown) as Box<dyn ElementFilter>]);
        assert!(facts.has_filters);
        assert!(!facts.replay_safe);
        assert!(!facts.preserves_order);
    }

    #[cfg(feature = "skyfilter")]
    #[test]
    fn transform_facts_trust_declared_filters() {
        let filter = auto_parse_filter(&format!("SkyFilter v{}\n\nCOMMIT", VERSION)).unwrap();
        let facts = transform_facts(&[filter]);
        assert!(facts.has_filters);
        assert!(facts.replay_safe);
        assert!(facts.preserves_order);
    }
}
