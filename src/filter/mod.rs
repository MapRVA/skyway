//! Filters/transforms OSM data.

mod cel;
mod osmfilter;

use cel::compile_cel_filter;
use indicatif::ProgressBar;
use osmfilter::parse::parse_filter;
use std::sync::mpsc::{Receiver, Sender};

use crate::elements::Element;

/// Represents a filter that can be evaluated on an `Element`, transforming it.
pub trait ElementFilter: Send {
    fn evaluate(&self, element: &mut Element) -> bool;
}

pub fn create_filter(filter_contents: &str) -> Box<dyn ElementFilter> {
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

/// Filters OSM data.
///
/// * `filter_contents`: A textual representation of the filter, usually read in from a file.
/// * `receiver`: Receiver for a channel of `Element`s.
/// * `sender`: Sender for a channel of `Element`s.
/// * `progress`: The ProgressBar for this read operation.
pub fn filter_elements(
    filter: Box<dyn ElementFilter>,
    receiver: Receiver<Element>,
    sender: Sender<Element>,
    progress: ProgressBar,
) {
    progress.set_message("Filtering elements...");
    let progress_clone = progress.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(100));
        progress_clone.tick();
        if progress_clone.is_finished() {
            break;
        }
    });

    for mut e in receiver.iter() {
        if filter.evaluate(&mut e) {
            let _ = sender.send(e);
        }
    }
    progress.finish_with_message("Filtering elements...done");
}
