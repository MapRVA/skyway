//! Writes OSM data out.

use std::{path::PathBuf, sync::mpsc::Receiver};

use crate::{SkywayError, chunks::ElementChunk, elements::Metadata};

// other features will depend on this someday, so I'm keeping it separate
#[cfg(feature = "geojson")]
mod geo;

#[cfg(feature = "geojson")]
mod geojson;
#[cfg(feature = "geojson")]
pub use geojson::GeoJsonWriter;

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

/// Convert an i32 representing decimicrodegrees (10⁻⁷) to an f64 representing degrees.
pub fn coord_to_f64(coord: i32) -> f64 {
    return (coord as f64) / 1e7;
}

/// Convert an i32 representing decimicrodegrees (10⁻⁷) to a String representing degrees.
pub fn coord_to_string(coord: i32) -> String {
    lexical::to_string(coord_to_f64(coord))
}

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

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_coord_to_string() {
        let input1 = 85857061 as i32;
        let output1 = String::from("8.5857061");
        assert_eq!(coord_to_string(input1), output1);

        let input2 = 502106895 as i32;
        let output2 = String::from("50.2106895");
        assert_eq!(coord_to_string(input2), output2);
    }
}
