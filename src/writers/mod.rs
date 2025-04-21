//! Writes OSM data out.

use std::{path::PathBuf, sync::mpsc::Receiver};

use crate::{SkywayError, chunks::ElementChunk, elements::Metadata};

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

/// `Writer` implements the output of OpenStreetMap data in a specific format.
pub trait Writer {
    /// Write data out from a `ParallelIterator` of `Chunk`s.
    ///
    /// * `par_iter`: Object implementing `IntoParallelIterator<Item = Chunk>`.
    /// * `metadata_receiver`: Receiver for a channel of (1) `Metadata`.\
    /// * `dest`: Path to output file. If None, data will be written to stdout.
    fn write(
        &self,
        element_receiver: Receiver<ElementChunk>,
        metadata_receiver: Receiver<Metadata>,
        dest: Option<PathBuf>,
    ) -> Result<(), SkywayError>;
}
