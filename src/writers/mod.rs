use std::sync::mpsc::Receiver;
use std::io::Write;

use crate::elements;

mod json;
use json::write_json;

pub fn write_file<D: Write>(reciever: Receiver<elements::Element>, metadata: elements::Metadata, to: &str, destination: D) {
    match to {
        "json" => write_json(reciever, metadata, destination),
        // Some("json") => load_json(file_path),
        _ => panic!("Output filetype not supported!")
    }
}
