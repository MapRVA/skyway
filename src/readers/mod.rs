//! Reads OSM data into skyway.

use std::fs;
use std::io::{stdin, BufReader, Read};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::mpsc::Sender;

use crate::elements::{Element, Metadata};
use crate::SkywayError;

mod json;
use json::JsonReader;

mod opl;
use opl::OplReader;

mod pbf;
use pbf::PbfReader;

mod xml;
use xml::XmlReader;

/// Enum that represents the different input file formats skyway supports.
#[derive(Debug)]
pub enum InputFileFormat {
    Json,
    Opl,
    Pbf,
    Xml,
}

impl FromStr for InputFileFormat {
    type Err = SkywayError;

    /// Converts a file extension `&str` into the appropriate InputFileFormat variant.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(InputFileFormat::Json),
            "opl" => Ok(InputFileFormat::Opl),
            "osm" => Ok(InputFileFormat::Xml),
            "pbf" => Ok(InputFileFormat::Pbf),
            "xml" => Ok(InputFileFormat::Xml),
            _ => Err(SkywayError::UnknownInputFormat),
        }
    }
}

pub trait Reader: Send {
    /// Reads data into skyway.
    ///
    /// * `sender`: Sender for a channel of `Element`s.
    /// * `metadata_sender`: Sender for a channel of (1) `Metadata`.
    fn read(&mut self, sender: Sender<Vec<Element>>, metadata_sender: Sender<Metadata>);
}

fn open_or_stdin(path: Option<PathBuf>) -> Box<dyn Read + Send> {
    match path {
        Some(p) => match fs::File::open(p) {
            Ok(f) => Box::new(f) as Box<dyn Read + Send>,
            Err(e) => panic!("Unable to open input file: {e:?}"),
        },
        None => Box::new(stdin()) as Box<dyn Read + Send>,
    }
}

pub fn generate_reader(from: InputFileFormat, path: Option<PathBuf>) -> Box<dyn Reader> {
    match from {
        InputFileFormat::Json => {
            let mut buffer = String::new();
            let mut source = open_or_stdin(path);
            let src = match source.read_to_string(&mut buffer) {
                Ok(_) => buffer,
                Err(e) => {
                    panic!("Error reading input: {e:?}");
                }
            };
            Box::new(JsonReader { src })
        }
        InputFileFormat::Opl => Box::new(OplReader {
            src: Box::new(BufReader::new(open_or_stdin(path))),
        }),
        InputFileFormat::Pbf => Box::new(PbfReader {
            src: Box::new(BufReader::new(open_or_stdin(path))),
        }),
        InputFileFormat::Xml => {
            let mut buffer = String::new();
            let mut source = open_or_stdin(path);
            let src = match source.read_to_string(&mut buffer) {
                Ok(_) => buffer,
                Err(e) => {
                    panic!("Error reading input: {e:?}");
                }
            };
            Box::new(XmlReader { src })
        }
    }
}

pub fn get_reader(input: Option<&str>, from: InputFileFormat) -> Box<dyn Reader> {
    match input {
        None => generate_reader(from, None),
        Some(a) => generate_reader(from, Some(PathBuf::from(a))),
    }
}
