//! Reads OSM data into skyway.

use clap::ValueEnum;
use enum_dispatch::enum_dispatch;

use std::{
    fs,
    io::{stdin, BufRead, BufReader, Read},
    path::PathBuf,
    sync::mpsc::Sender,
    thread,
};

use crate::{
    chunks::{Chunk, ChunkBuilder},
    elements::Metadata,
    FileFormatOptions, SkywayError,
};

#[cfg(feature = "json")]
pub mod json;
#[cfg(feature = "json")]
use json::JsonReader;

#[cfg(feature = "opl")]
pub mod opl;
#[cfg(feature = "opl")]
use opl::OplReader;

#[cfg(feature = "osmx")]
pub mod osmx;

#[cfg(feature = "pbf")]
pub mod pbf;
#[cfg(feature = "pbf")]
use pbf::PbfReader;

#[cfg(feature = "xml")]
pub mod xml;
#[cfg(feature = "xml")]
use xml::XmlReader;

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

#[enum_dispatch]
pub enum Readers {
    #[cfg(feature = "json")]
    JsonReader,
    #[cfg(feature = "opl")]
    OplReader,
    #[cfg(feature = "pbf")]
    PbfReader,
    #[cfg(feature = "xml")]
    XmlReader,
}

impl InputFileFormat {
    pub fn generate_reader(self) -> Readers {
        match self {
            #[cfg(feature = "json")]
            InputFileFormat::Json => Readers::JsonReader(JsonReader::new()),
            #[cfg(feature = "opl")]
            InputFileFormat::Opl => Readers::OplReader(OplReader::new()),
            #[cfg(feature = "pbf")]
            InputFileFormat::Pbf => Readers::PbfReader(PbfReader::new()),
            #[cfg(feature = "xml")]
            InputFileFormat::Xml => Readers::XmlReader(XmlReader::new()),
        }
    }
}

impl FileFormatOptions for InputFileFormat {
    fn format_error(ext: &str) -> SkywayError {
        SkywayError::UnknownInputFormat(ext.to_string())
    }
}

pub fn open(path: PathBuf) -> Box<dyn Read + Send> {
    match fs::File::open(path) {
        Ok(f) => Box::new(f) as Box<dyn Read + Send>,
        Err(e) => panic!("Unable to open input file: {e:?}"),
    }
}

pub fn get_reader(src: Option<PathBuf>) -> Box<dyn BufRead + Send> {
    Box::new(BufReader::new(match src {
        Some(path) => open(path),
        None => Box::new(stdin()),
    }))
}

#[enum_dispatch(Readers)]
pub trait Reader: Send + 'static {
    /// Create a new instance of this Reader

    /// Reads data into skyway.
    ///
    /// * `sender`: Sender for a channel of `Element`s.
    /// * `metadata_sender`: Sender for a channel of (1) `Metadata`.
    fn read_file(
        self,
        src: Option<PathBuf>,
        metadata_sender: Sender<Metadata>,
        chunk_builder: ChunkBuilder,
        write_thread: thread::JoinHandle<()>,
        final_iterator: impl Fn(Chunk) + Sync,
    );
}
