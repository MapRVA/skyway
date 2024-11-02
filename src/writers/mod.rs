//! Writes OSM data out.

use indicatif::ProgressBar;
use std::io::Write;
use std::str::FromStr;
use std::sync::mpsc::Receiver;

use crate::elements::{Element, Metadata};
use crate::SkywayError;

#[cfg(feature = "json")]
mod json;

#[cfg(feature = "o5m")]
mod o5m;

#[cfg(feature = "opl")]
mod opl;

#[cfg(feature = "xml")]
mod xml;

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
            // #[cfg(feature = "o5m")]
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

    #[allow(unreachable_patterns)]
    match to {
        #[cfg(feature = "json")]
        OutputFileFormat::Json => json::write_json(receiver, metadata, destination, false),
        //#[cfg(feature = "o5m")]
        // OutputFileFormat::O5m => o5m::write_o5m(reciever, metadata, destination),
        #[cfg(feature = "opl")]
        OutputFileFormat::Opl => opl::write_opl(receiver, metadata, destination),
        #[cfg(feature = "json")]
        OutputFileFormat::Overpass => json::write_json(receiver, metadata, destination, true),
        #[cfg(feature = "xml")]
        OutputFileFormat::Xml => xml::write_xml(receiver, metadata, destination),
        _ => panic!("Feature not enabled for output format {:?}", to),
    }

    progress.finish_with_message("Writing output...done");
}
