//! Reads OSM data into skyway.

use rayon::prelude::*;

use std::{
    fs,
    io::{BufRead, BufReader, Read, stdin},
    path::PathBuf,
    sync::mpsc::Sender,
};

use crate::{
    chunks::{ChunkBuilder, ElementChunk},
    elements::Metadata,
};

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

/// Convert f64 coordinate representing degrees into i32 representing decimicrodegrees (10⁻⁷)
fn coord_from_f64(value: &f64) -> i32 {
    (value * 1e7).round() as i32
}

pub trait Reader: Sized + Clone + Send + 'static {
    /// Reads data into skyway.
    ///
    /// * `src`: Path of input file, None if the input is standard input.
    /// * `metadata_sender`: Sender for a channel of (1) `Metadata`.
    /// * `chunk_builder`: `ChunkBuilder` for building Chunks as elements are read.
    ///
    /// # Chunk index contract
    ///
    /// Chunk indices must be contiguous from 0 and assigned in the
    /// sequential stage of reading, before work is handed to Rayon. The
    /// returned parallel iterator must be `par_bridge()` over an iterator
    /// that yields chunks (or the raw input each chunk is decoded from) in
    /// ascending index order, not `into_par_iter()` on a collection: Rayon's
    /// bridge pulls one item at a time, so workers pick up indices in order.
    /// The parallel runner's back-pressure relies on this to avoid deadlock.
    /// A reader that violates it can park every worker forever.
    fn read_file(
        self,
        src: Option<PathBuf>,
        metadata_sender: Sender<Metadata>,
        chunk_builder: ChunkBuilder,
    ) -> impl ParallelIterator<Item = ElementChunk>;
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_coord_from_f64() {
        let input1 = 8.5857061 as f64;
        let output1 = 85857061 as i32;
        assert_eq!(coord_from_f64(&input1), output1);

        let input2 = 50.2106895 as f64;
        let output2 = 502106895 as i32;
        assert_eq!(coord_from_f64(&input2), output2);
    }
}
