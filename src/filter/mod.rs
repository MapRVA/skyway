mod osmfilter;

use osmfilter::parse::parse_filter;
use std::sync::mpsc::{Sender, Receiver};

use crate::elements;

pub trait ElementFilter {
    fn evaluate(&self, element: elements::Element) -> Option<elements::Element>;
}

pub fn filter_elements(filter_contents: &str, receiver: Receiver<elements::Element>, sender: Sender<elements::Element>) {
    let filter = parse_filter(filter_contents);
    for e in receiver.iter() {
        if let Some(v) = filter.evaluate(e) {
            let _ = sender.send(v);
        }
    }
}
