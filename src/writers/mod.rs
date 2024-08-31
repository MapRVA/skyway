use std::io::Write;
use std::sync::mpsc::Receiver;

use crate::elements::{Element, Metadata};

mod json;
use json::write_json;

mod xml;
use xml::write_xml;

pub fn write_file<D: Write>(
    reciever: Receiver<Element>,
    metadata: Metadata,
    to: &str,
    destination: D,
) {
    match to {
        "json" => write_json(reciever, metadata, destination),
        "xml" => write_xml(reciever, metadata, destination),
        _ => panic!("Output filetype not supported!"),
    }
}
