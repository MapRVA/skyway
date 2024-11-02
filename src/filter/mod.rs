//! Filters/transforms OSM data.

#[cfg(feature = "cel")]
mod cel;
#[cfg(feature = "osmfilter")]
mod osmfilter;

#[cfg(feature = "cel")]
use cel::compile_cel_filter;
use indicatif::ProgressBar;
#[cfg(feature = "osmfilter")]
use osmfilter::parse::parse_filter;
use std::sync::mpsc::{Receiver, Sender};

use crate::elements::Element;

/// Represents a filter that can be evaluated on an `Element`, transforming it.
pub trait ElementFilter: Send {
    fn evaluate(&self, element: &mut Element) -> bool;
}

pub fn create_filter(filter_contents: &str) -> Box<dyn ElementFilter> {
    #[cfg(feature = "osmfilter")]
    if let Some(f) = parse_filter(filter_contents) {
        return Box::new(f);
    }

    #[cfg(feature = "cel")]
    if let Some(f) = compile_cel_filter(filter_contents) {
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
    receiver: Receiver<Vec<Element>>,
    sender: Sender<Vec<Element>>,
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

    receiver
        .iter()
        .map(|c| {
            let mut keep_elements = Vec::new();
            for mut element in c {
                if filter.evaluate(&mut element) {
                    keep_elements.push(element);
                }
            }
            keep_elements
        })
        .for_each(|c| sender.send(c).expect("Unable to send element to channel"));
    progress.finish_with_message("Filtering elements...done");
}
