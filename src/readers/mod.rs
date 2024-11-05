//! Reads OSM data into skyway.

use std::fs;
use std::io::{stdin, Read};
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
    #[cfg(feature = "json")]
    #[value(name = "json")]
    Json,
    #[cfg(feature = "opl")]
    #[value(name = "opl")]
    Opl,
    #[cfg(feature = "pbf")]
    #[value(name = "pbf")]
    Pbf,
    #[cfg(feature = "xml")]
    #[value(name = "xml", alias = "osm")]
    Xml,
}

impl FileFormatOptions for InputFileFormat {
    fn format_error(ext: &str) -> SkywayError {
        SkywayError::UnknownInputFormat(ext.to_string())
    }
}

impl InputFileFormat {
    pub fn generate_reader(self, src: Box<dyn Read + Send>) -> Box<dyn Reader> {
        match self {
            #[cfg(feature = "json")]
            InputFileFormat::Json => Box::new(json::JsonReader::new(src)),
            #[cfg(feature = "opl")]
            InputFileFormat::Opl => Box::new(opl::OplReader::new(src)),
            #[cfg(feature = "pbf")]
            InputFileFormat::Pbf => Box::new(pbf::PbfReader::new(src)),
            #[cfg(feature = "xml")]
            InputFileFormat::Xml => Box::new(xml::XmlReader::new(src)),
        }
    }
}

pub fn open(path: PathBuf, no_overwrite: bool) -> Result<Box<dyn Read + Send>, SkywayError> {
    if no_overwrite && path.exists() {
        return Err(SkywayError::OutputFileExists);
    }

    match fs::File::open(path) {
        Ok(f) => Ok(Box::new(f) as Box<dyn Read + Send>),
        Err(e) => panic!("Unable to open input file: {e:?}"),
    }
}

pub fn open_or_stdin(
    path: Option<PathBuf>,
    no_overwrite: bool,
) -> Result<Box<dyn Read + Send>, SkywayError> {
    match path {
        Some(p) => open(p, no_overwrite),
        None => Ok(Box::new(stdin()) as Box<dyn Read + Send>),
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
