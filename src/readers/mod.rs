//! Reads OSM data into skyway.

use indicatif::ProgressBar;
use std::io::{BufReader, Read};
use std::str::FromStr;
use std::sync::mpsc::Sender;

use crate::elements;
use crate::SkywayError;

mod json;
use json::read_json;

mod opl;
use opl::read_opl;

mod pbf;
use pbf::read_pbf;

mod xml;
use xml::read_xml;

/// Enum that represents the different input file formats skyway supports.
#[derive(Debug)]
pub enum InputFileFormat {
    Json,
    Opl,
    Pbf,
    Xml,
}

impl FromStr for InputFileFormat {
    type Err = SkywayError;

    /// Converts a file extension `&str` into the appropriate InputFileFormat variant.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(InputFileFormat::Json),
            "opl" => Ok(InputFileFormat::Opl),
            "pbf" => Ok(InputFileFormat::Pbf),
            "xml" => Ok(InputFileFormat::Xml),
            _ => Err(SkywayError::UnknownInputFormat),
        }
    }
}

/// Reads data into skyway.
///
/// * `sender`: Sender for a channel of `Element`s.
/// * `metadata_sender`: Sender for a channel of (1) `Metadata`.
/// * `from`: File format to parse.
/// * `source`: Input data source.
/// * `progress`: The ProgressBar for this read operation.
pub fn read_file<S: Read + Send>(
    sender: Sender<Vec<elements::Element>>,
    metadata_sender: Sender<elements::Metadata>,
    from: InputFileFormat,
    mut source: S,
    progress: ProgressBar,
) {
    progress.set_message("Reading input...");
    let progress_clone = progress.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(100));
        progress_clone.tick();
        if progress_clone.is_finished() {
            break;
        }
    });

    match from {
        InputFileFormat::Json => {
            let mut buffer = String::new();
            let source_str = match source.read_to_string(&mut buffer) {
                Ok(_) => buffer.as_str(),
                Err(e) => {
                    panic!("Error reading input: {e:?}");
                }
            };
            read_json(sender, metadata_sender, source_str);
        }
        InputFileFormat::Opl => {
            let reader = BufReader::new(source);
            read_opl(sender, metadata_sender, reader);
        }
        InputFileFormat::Pbf => read_pbf(sender, metadata_sender, source),
        InputFileFormat::Xml => {
            let mut buffer = String::new();
            let source_str = match source.read_to_string(&mut buffer) {
                Ok(_) => buffer.as_str(),
                Err(e) => {
                    panic!("Error reading input: {e:?}");
                }
            };
            read_xml(sender, metadata_sender, source_str);
        }
    }
    progress.finish_with_message("Reading input...done");
}
