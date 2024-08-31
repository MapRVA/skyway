mod cel;
mod osmfilter;

use osmfilter::parse::parse_filter;
use osmfilter::logic::OsmFilter;
use cel::{CelFilter, compile_cel_filter};
use std::sync::mpsc::{Sender, Receiver};

use crate::elements;

enum Filter {
    OsmFilter(OsmFilter),
    CelFilter(CelFilter),
}

impl Filter {
    fn evaluate(&self, element: elements::Element) -> Option<elements::Element> {
        match self {
            Filter::OsmFilter(f) => {
                f.evaluate(element)
            },
            Filter::CelFilter(f) => {
                f.evaluate(element)
            }
        }
    }
}

fn _create_filter(filter_contents: &str) -> Filter {
    let osmfilter = parse_filter(filter_contents);
    if let Some(f) = osmfilter {
        return Filter::OsmFilter(f);
    }
    let celfilter = compile_cel_filter(filter_contents);
    if let Some(f) = celfilter {
        return Filter::CelFilter(f);
    }
    panic!("Unable to parse filter: {filter_contents:?}");
}

pub fn filter_elements(filter_contents: &str, receiver: Receiver<elements::Element>, sender: Sender<elements::Element>) {
    let filter = _create_filter(filter_contents);
    for e in receiver.iter() {
        if let Some(v) = filter.evaluate(e) {
            let _ = sender.send(v);
        }
    }
}
