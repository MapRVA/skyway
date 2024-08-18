use std::sync::mpsc::Sender;
use std::io::Read;

use crate::elements;

mod pbf;
use pbf::read_pbf;

// mod json;
// use json::read_json;


pub fn read_file<S: Read + Send>(sender: Sender<elements::Element>, from: &str, source: S) {
    match from {
        "pbf" => read_pbf(sender, source),
        // "json" => read_json(file_path),
        _ => panic!("Filetype not supported!")
    }
}
