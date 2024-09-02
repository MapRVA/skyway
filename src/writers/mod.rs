use std::io::Write;
use std::str::FromStr;
use std::sync::mpsc::Receiver;

use crate::elements::{Element, Metadata};
use crate::SkywayError;

mod json;
use json::write_json;

mod xml;
use xml::write_xml;

#[derive(Debug)]
pub enum OutputFileFormat {
    Json,
    Xml,
}

impl FromStr for OutputFileFormat {
    type Err = SkywayError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "xml" => Ok(OutputFileFormat::Xml),
            // TODO: recognize JSON, but warn user that it may be confused for Overpass JSON
            "json" => Ok(OutputFileFormat::Json),
            _ => Err(SkywayError::UnknownOutputFormat),
        }
    }
}

pub fn write_file<D: Write>(
    reciever: Receiver<Element>,
    metadata: Metadata,
    to: OutputFileFormat,
    destination: D,
) {
    match to {
        OutputFileFormat::Json => write_json(reciever, metadata, destination),
        OutputFileFormat::Xml => write_xml(reciever, metadata, destination),
    }
}
