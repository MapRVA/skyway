//! Reads OSM data into skyway.

use rayon::prelude::*;

#[cfg(feature = "cli")]
use clap::ValueEnum;

use std::{
    fs,
    io::{stdin, BufRead, BufReader, Read},
    path::PathBuf,
    sync::mpsc::{channel, Receiver, Sender},
    thread,
};

use crate::{
    chunks::{ChunkBuilder, ElementChunk},
    elements::Metadata,
    writers::*,
    SkywayError,
};

#[cfg(feature = "filter")]
use crate::filter::{build_filter, ElementFilter};

#[cfg(not(feature = "filter"))]
use std::convert::identity;

#[cfg(feature = "cli")]
use crate::FileFormatOptions;

#[cfg(feature = "json")]
mod json;
#[cfg(feature = "json")]
pub use json::JsonReader;

#[cfg(feature = "opl")]
mod opl;
#[cfg(feature = "opl")]
pub use opl::OplReader;

#[cfg(feature = "osmx")]
mod osmx;

#[cfg(feature = "pbf")]
mod pbf;
#[cfg(feature = "pbf")]
pub use pbf::PbfReader;

#[cfg(feature = "xml")]
mod xml;
#[cfg(feature = "xml")]
pub use xml::XmlReader;

/// Enum that represents the different input file formats skyway supports.
#[cfg(feature = "cli")]
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

#[cfg(feature = "cli")]
impl FileFormatOptions for InputFileFormat {
    fn format_error(ext: &str) -> SkywayError {
        SkywayError::UnknownInputFormat(ext.to_string())
    }
}

/// Enum that represents the different input file formats skyway supports.
#[cfg(not(feature = "cli"))]
#[derive(Clone, Debug, PartialEq)]
pub enum InputFileFormat {
    #[cfg(feature = "json")]
    Json,
    #[cfg(feature = "opl")]
    Opl,
    #[cfg(feature = "pbf")]
    Pbf,
    #[cfg(feature = "xml")]
    Xml,
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

fn transform_metadata(
    metadata_receiver: Receiver<Metadata>,
    metadata_sender: Sender<Metadata>,
    preserve_generator: bool,
) {
    let mut metadata = metadata_receiver.into_iter().next().unwrap();
    if !preserve_generator {
        metadata.generator = Some(format!("skyway v{}", env!("CARGO_PKG_VERSION")))
    }
    metadata_sender.send(metadata).unwrap();
}

pub trait Reader: Sized {
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
    ) -> impl ParallelIterator<Item = ElementChunk>;

    fn run_conversion(
        self,
        source: Option<PathBuf>,
        chunk_size: usize,
        #[cfg(feature = "filter")] filters: Vec<Box<dyn ElementFilter>>,
        output_format: OutputFileFormat,
        dest: Option<PathBuf>,
        preserve_generator: bool,
    ) -> Result<(), SkywayError> {
        let (metadata_sender, metadata_receiver) = channel();
        let chunk_builder = ChunkBuilder::new(chunk_size);

        // any intermediate metadata transformations should happen here
        let (trans_metadata_sender, trans_metadata_receiver) = channel();
        thread::spawn(move || {
            transform_metadata(metadata_receiver, trans_metadata_sender, preserve_generator)
        });

        #[cfg(feature = "filter")]
        let combined_filter = build_filter(filters);
        #[cfg(feature = "filter")]
        let chunk_iterator = self
            .read_file(source, metadata_sender, chunk_builder)
            .map(|chunk| combined_filter(chunk));

        #[cfg(not(feature = "filter"))]
        let chunk_iterator = self.read_file(source, metadata_sender, chunk_builder);

        #[allow(unreachable_patterns)]
        match output_format {
            #[cfg(feature = "json")]
            OutputFileFormat::Json => {
                JsonWriter { overpass: false }.write(chunk_iterator, trans_metadata_receiver, dest)
            }
            #[cfg(feature = "o5m")]
            OutputFileFormat::O5m => {
                O5mWriter {}.write(chunk_iterator, trans_metadata_receiver, dest)
            }
            #[cfg(feature = "opl")]
            OutputFileFormat::Opl => {
                OplWriter {}.write(chunk_iterator, trans_metadata_receiver, dest)
            }
            #[cfg(feature = "json")]
            OutputFileFormat::Overpass => {
                JsonWriter { overpass: true }.write(chunk_iterator, trans_metadata_receiver, dest)
            }
            #[cfg(feature = "xml")]
            OutputFileFormat::Xml => {
                XmlWriter {}.write(chunk_iterator, trans_metadata_receiver, dest)
            }
            _ => Err(SkywayError::UnexpectedError(
                "A file conversion was attempted with an unknown output format.".to_owned(),
            )),
        }
    }
}
