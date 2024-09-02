mod cel;
mod osmfilter;

use cel::compile_cel_filter;
use osmfilter::parse::parse_filter;
use std::sync::mpsc::{Receiver, Sender};

use crate::elements::Element;

pub trait ElementFilter {
    fn evaluate(&self, element: Element) -> Option<Element>;
}

fn _create_filter(filter_contents: &str) -> Box<dyn ElementFilter> {
    let osmfilter = parse_filter(filter_contents);
    if let Some(f) = osmfilter {
        return Box::new(f);
    }
    let celfilter = compile_cel_filter(filter_contents);
    if let Some(f) = celfilter {
        return Box::new(f);
    }
    panic!("Unable to parse filter: {filter_contents:?}");
}

pub fn filter_elements(
    filter_contents: &str,
    receiver: Receiver<Element>,
    sender: Sender<Element>,
) {
    let filter = _create_filter(filter_contents);
    for e in receiver.iter() {
        if let Some(v) = filter.evaluate(e) {
            let _ = sender.send(v);
        }
    }
}
