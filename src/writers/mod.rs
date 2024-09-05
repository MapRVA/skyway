use std::io::Write;
use std::str::FromStr;
use std::sync::mpsc::Receiver;

use crate::elements::{Element, Metadata};
use crate::SkywayError;

mod json;
use json::write_json;

// mod o5m;
// use o5m::write_o5m;

mod opl;
use opl::write_opl;

mod xml;
use xml::write_xml;

#[derive(Debug)]
pub enum OutputFileFormat {
    Json,
    // O5m,
    Opl,
    Xml,
}

impl FromStr for OutputFileFormat {
    type Err = SkywayError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            // TODO: recognize JSON, but warn user that it may be confused for Overpass JSON
            "json" => Ok(OutputFileFormat::Json),
            // "o5m" => Ok(OutputFileFormat::O5m),
            "opl" => Ok(OutputFileFormat::Opl),
            "xml" => Ok(OutputFileFormat::Xml),
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
        // OutputFileFormat::O5m => write_o5m(reciever, metadata, destination),
        OutputFileFormat::Opl => write_opl(reciever, metadata, destination),
        OutputFileFormat::Xml => write_xml(reciever, metadata, destination),
    }
}
