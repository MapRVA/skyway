//! Writes OSM data out.

use rayon::prelude::*;

#[cfg(feature = "cli")]
use clap::ValueEnum;

use std::{path::PathBuf, sync::Arc};

use crate::{chunks::ElementChunk, elements::Metadata, SkywayError};

#[cfg(feature = "cli")]
use crate::FileFormatOptions;

#[cfg(feature = "json")]
mod json;
#[cfg(feature = "json")]
pub use json::JsonWriter;

#[cfg(feature = "o5m")]
mod o5m;
#[cfg(feature = "o5m")]
pub use o5m::O5mWriter;

#[cfg(feature = "opl")]
mod opl;
#[cfg(feature = "opl")]
pub use opl::OplWriter;

#[cfg(feature = "xml")]
mod xml;
#[cfg(feature = "xml")]
pub use xml::XmlWriter;

/// Enum that represents the different output file formats skyway supports.
#[cfg(feature = "cli")]
#[derive(Clone, Debug, ValueEnum)]
pub enum OutputFileFormat {
    #[cfg(feature = "json")]
    #[value(name = "json")]
    Json,
    #[cfg(feature = "o5m")]
    #[value(name = "o5m")]
    O5m,
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

#[cfg(feature = "cli")]
impl FileFormatOptions for OutputFileFormat {
    fn format_error(ext: &str) -> SkywayError {
        SkywayError::UnknownOutputFormat(ext.to_string())
    }
}

/// Enum that represents the different output file formats skyway supports.
#[cfg(not(feature = "cli"))]
#[derive(Clone, Debug)]
pub enum OutputFileFormat {
    #[cfg(feature = "json")]
    Json,
    #[cfg(feature = "o5m")]
    O5m,
    #[cfg(feature = "opl")]
    Opl,
    #[cfg(feature = "json")]
    Overpass,
    #[cfg(feature = "xml")]
    Xml,
}

/// `Writer` implements the output of OpenStreetMap data in a specific format.
pub trait Writer {
    /// Write data out from a `ParallelIterator` of `Chunk`s.
    ///
    /// * `par_iter`: Object implementing `IntoParallelIterator<Item = Chunk>`.
    /// * `metadata_receiver`: Receiver for a channel of (1) `Metadata`.\
    /// * `dest`: Path to output file. If None, data will be written to stdout.
    fn write<I>(
        &self,
        par_iter: I,
        metadata: Arc<Metadata>,
        dest: Option<PathBuf>,
    ) -> Result<(), SkywayError>
    where
        I: IntoParallelIterator<Item = ElementChunk>;
}
