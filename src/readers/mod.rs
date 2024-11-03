//! Reads OSM data into skyway.

use std::fs;
use std::io::{stdin, BufReader, Read};
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use clap::ValueEnum;

use crate::{
    chunks::{Chunk, ChunkBuilder},
    elements::Metadata,
    FileFormatOptions, SkywayError,
};

#[cfg(feature = "json")]
mod json;

#[cfg(feature = "opl")]
mod opl;

#[cfg(feature = "osmx")]
mod osmx;

#[cfg(feature = "pbf")]
mod pbf;

#[cfg(feature = "xml")]
mod xml;

/// Enum that represents the different input file formats skyway supports.
#[derive(Clone, Debug, PartialEq, ValueEnum)]
pub enum InputFileFormat {
    #[value(name = "json")]
    Json,
    #[value(name = "opl")]
    Opl,
    #[value(name = "pbf")]
    Pbf,
    #[value(name = "xml", alias = "osm")]
    Xml,
}

impl FileFormatOptions for InputFileFormat {
    fn format_error(ext: &str) -> SkywayError {
        SkywayError::UnknownInputFormat(ext.to_string())
    }
}

impl InputFileFormat {
    pub fn generate_reader(self, path: Option<PathBuf>) -> Box<dyn Reader> {
        #[allow(unreachable_patterns)]
        match self {
            #[cfg(feature = "json")]
            InputFileFormat::Json => {
                let mut buffer = String::new();
                let mut source = open_or_stdin(path);
                let src = match source.read_to_string(&mut buffer) {
                    Ok(_) => buffer,
                    Err(e) => {
                        panic!("Error reading input: {e:?}");
                    }
                };
                Box::new(json::JsonReader { src })
            }
            #[cfg(feature = "opl")]
            InputFileFormat::Opl => Box::new(opl::OplReader {
                src: Box::new(BufReader::new(open_or_stdin(path))),
            }),
            #[cfg(feature = "pbf")]
            InputFileFormat::Pbf => Box::new(pbf::PbfReader {
                src: Box::new(BufReader::new(open_or_stdin(path))),
            }),
            #[cfg(feature = "xml")]
            InputFileFormat::Xml => {
                let mut buffer = String::new();
                let mut source = open_or_stdin(path);
                let src = match source.read_to_string(&mut buffer) {
                    Ok(_) => buffer,
                    Err(e) => {
                        panic!("Error reading input: {e:?}");
                    }
                };
                Box::new(xml::XmlReader { src })
            }
            _ => panic!("Feature not enabled for input format {:?}", self),
        }
    }
}

pub trait Reader: Send {
    /// Reads data into skyway.
    ///
    /// * `sender`: Sender for a channel of `Element`s.
    /// * `metadata_sender`: Sender for a channel of (1) `Metadata`.
    fn read(
        &mut self,
        chunk_builder: ChunkBuilder,
        sender: Sender<Chunk>,
        metadata_sender: Sender<Metadata>,
    );
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
