mod osmfilter;

use osmfilter::parse::parse_filter;
use std::sync::mpsc::{Sender, Receiver};

use crate::elements;

pub trait ElementFilter {
    fn evaluate(&self, element: elements::Element) -> Option<elements::Element>;
}


fn _create_filter(filter_contents: &str) -> impl ElementFilter {
    let osmfilter = parse_filter(filter_contents);
    if let Some(f) = osmfilter {
        return f;
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
