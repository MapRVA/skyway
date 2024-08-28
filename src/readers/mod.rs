use std::sync::mpsc::Sender;
use std::io::Read;

use crate::elements;

mod json;
use json::read_json;

mod pbf;
use pbf::read_pbf;

mod xml;
use xml::read_xml;

pub fn read_file<S: Read + Send>(sender: Sender<elements::Element>, metadata_sender: Sender<elements::Metadata>, from: &str, mut source: S) {
    match from {
        "json" => {
            let mut buffer = String::new();
            let source_str = match source.read_to_string(&mut buffer) {
                Ok(_) => buffer.as_str(),
                Err(e) => {
                    panic!("Error reading input: {e:?}");
                },
            };
            read_json(sender, metadata_sender, source_str);
        },
        "pbf" => read_pbf(sender, metadata_sender, source),
        "xml" => {
            let mut buffer = String::new();
            let source_str = match source.read_to_string(&mut buffer) {
                Ok(_) => buffer.as_str(),
                Err(e) => {
                    panic!("Error reading input: {e:?}");
                },
            };
            read_xml(sender, metadata_sender, source_str);
        },
        _ => panic!("Filetype not supported!")
    }
}
