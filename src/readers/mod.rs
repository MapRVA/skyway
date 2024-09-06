use std::io::{BufReader, Read};
use std::str::FromStr;
use std::sync::mpsc::Sender;

use crate::elements;
use crate::SkywayError;

mod json;
use json::read_json;

mod opl;
use opl::read_opl;

mod pbf;
use pbf::read_pbf;

mod xml;
use xml::read_xml;

#[derive(Debug)]
pub enum InputFileFormat {
    Json,
    Opl,
    Pbf,
    Xml,
}

impl FromStr for InputFileFormat {
    type Err = SkywayError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(InputFileFormat::Json),
            "opl" => Ok(InputFileFormat::Opl),
            "pbf" => Ok(InputFileFormat::Pbf),
            "xml" => Ok(InputFileFormat::Xml),
            _ => Err(SkywayError::UnknownInputFormat),
        }
    }
}

pub fn read_file<S: Read + Send>(
    sender: Sender<elements::Element>,
    metadata_sender: Sender<elements::Metadata>,
    from: InputFileFormat,
    mut source: S,
) {
    match from {
        InputFileFormat::Json => {
            let mut buffer = String::new();
            let source_str = match source.read_to_string(&mut buffer) {
                Ok(_) => buffer.as_str(),
                Err(e) => {
                    panic!("Error reading input: {e:?}");
                }
            };
            read_json(sender, metadata_sender, source_str);
        }
        InputFileFormat::Opl => {
            let reader = BufReader::new(source);
            read_opl(sender, metadata_sender, reader);
        }
        InputFileFormat::Pbf => read_pbf(sender, metadata_sender, source),
        InputFileFormat::Xml => {
            let mut buffer = String::new();
            let source_str = match source.read_to_string(&mut buffer) {
                Ok(_) => buffer.as_str(),
                Err(e) => {
                    panic!("Error reading input: {e:?}");
                }
            };
            read_xml(sender, metadata_sender, source_str);
        }
    }
}
