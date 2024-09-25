//! Writes OSM data out.

use indicatif::ProgressBar;
use std::io::Write;
use std::str::FromStr;
use std::sync::mpsc::Receiver;

use crate::elements::{Element, Metadata};
use crate::SkywayError;

mod json;
use json::write_json;

mod o5m;
use o5m::write_o5m;

mod opl;
use opl::write_opl;

mod xml;
use xml::write_xml;

/// Enum that represents the different output file formats skyway supports.
#[derive(Debug)]
pub enum OutputFileFormat {
    Json,
    // O5m,
    Opl,
    Overpass,
    Xml,
}

impl FromStr for OutputFileFormat {
    type Err = SkywayError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            // TODO: recognize JSON, but warn user that it may be confused for Overpass JSON
            "json" => Ok(OutputFileFormat::Json),
            // "o5m" => Ok(OutputFileFormat::O5m),
            "opl" => Ok(OutputFileFormat::Opl),
            "osm" => Ok(OutputFileFormat::Xml),
            "overpass" => Ok(OutputFileFormat::Overpass),
            "xml" => Ok(OutputFileFormat::Xml),
            _ => Err(SkywayError::UnknownOutputFormat),
        }
    }
}

/// Writes data out.
///
/// * `receiver`: Receiver for a channel of `Element`s.
/// * `metadata_sender`: Document-level metadata.
/// * `to`: File format to write.
/// * `destination`: Output data destination.
/// * `progress`: The ProgressBar for this write operation.
pub fn write_file<D: Write>(
    receiver: Receiver<Vec<Element>>,
    metadata: Metadata,
    to: OutputFileFormat,
    destination: D,
    progress: ProgressBar,
) {
    progress.set_message("Writing output...");
    let progress_clone = progress.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(100));
        progress_clone.tick();
        if progress_clone.is_finished() {
            break;
        }
    });

    match to {
        OutputFileFormat::Json => write_json(receiver, metadata, destination, false),
        // OutputFileFormat::O5m => write_o5m(reciever, metadata, destination),
        OutputFileFormat::Opl => write_opl(receiver, metadata, destination),
        OutputFileFormat::Overpass => write_json(receiver, metadata, destination, true),
        OutputFileFormat::Xml => write_xml(receiver, metadata, destination),
    }

    progress.finish_with_message("Writing output...done");
}
