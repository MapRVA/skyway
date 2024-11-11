//! Writes OSM data out.

use std::{path::PathBuf, sync::mpsc::Receiver, thread};

use clap::ValueEnum;

use crate::{chunks::Chunk, elements::Metadata, FileFormatOptions, SkywayError};

#[cfg(feature = "json")]
mod json;

#[cfg(feature = "o5m")]
mod o5m;

#[cfg(feature = "opl")]
mod opl;
#[cfg(feature = "opl")]
use opl::OplWriter;

#[cfg(feature = "xml")]
mod xml;
#[cfg(feature = "xml")]
use xml::XmlWriter;

/// Enum that represents the different output file formats skyway supports.
#[derive(Clone, Debug, ValueEnum)]
pub enum OutputFileFormat {
    #[cfg(feature = "json")]
    #[value(name = "json")]
    Json,
    // #[cfg(feature = "o5m")]
    // #[value(name = "o5m")]
    // O5m,
    #[cfg(feature = "opl")]
    #[value(name = "opl")]
    Opl,
    #[cfg(feature = "json")]
    #[value(name = "overpass")]
    Overpass,
    #[cfg(feature = "xml")]
    #[value(name = "xml", alias = "osm")]
    Xml,
}

impl OutputFileFormat {
    pub fn generate_writer(self) -> Box<dyn Writer> {
        #[allow(unreachable_patterns)]
        match self {
            #[cfg(feature = "json")]
            OutputFileFormat::Json => json::write_json(receiver, metadata, destination, false),
            //#[cfg(feature = "o5m")]
            // OutputFileFormat::O5m => o5m::write_o5m(reciever, metadata, destination),
            #[cfg(feature = "opl")]
            OutputFileFormat::Opl => Box::new(OplWriter::new()),
            #[cfg(feature = "json")]
            OutputFileFormat::Overpass => json::write_json(receiver, metadata, destination, true),
            #[cfg(feature = "xml")]
            OutputFileFormat::Xml => Box::new(XmlWriter::new()),
            _ => panic!("Feature not enabled for output format {:?}", self),
        }
    }
}

impl FileFormatOptions for OutputFileFormat {
    fn format_error(ext: &str) -> SkywayError {
        SkywayError::UnknownOutputFormat(ext.to_string())
    }
}

pub trait Writer {
    /// Writes data out.
    ///
    /// * `receiver`: Receiver for a channel of `Element`s.
    /// * `metadata_sender`: Document-level metadata.
    /// * `to`: File format to write.
    /// * `destination`: Output data destination.
    /// * `progress`: The ProgressBar for this write operation.
    fn write_file(
        &self,
        metadata_receiver: Receiver<Metadata>,
        destination: Option<PathBuf>,
    ) -> (Box<dyn Fn(Chunk) + Sync>, thread::JoinHandle<()>);
}
