use thiserror::Error;

#[cfg(feature = "cli")]
use clap::ValueEnum;

#[cfg(feature = "pbf")]
use log::warn;

use std::path::PathBuf;

pub mod chunks;
pub mod elements;
pub mod readers;
pub mod writers;

use readers::*;
use writers::OutputFileFormat;

// selective imports that deal with filters
#[cfg(feature = "filter")]
pub mod filter;
#[cfg(feature = "filter")]
use filter::ElementFilter;

// All errors skyway can return
// this is a work in progress
#[derive(Error, Debug)]
pub enum SkywayError {
    #[error("Cannot determine input file format: {0}")]
    UnknownInputFormat(String),
    #[error("Cannot determine output file format: {0}")]
    UnknownOutputFormat(String),
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("File already exits")]
    OutputFileExists,
    #[error("Invalid input file")]
    InvalidInputFile,
    #[error("Invalid filter file: {0}")]
    InvalidFilterFile(String),
    #[error("Cannot parse filter file: {0}")]
    UnparsableFilter(String),
    #[error("Unexpected error (this is a bug): {0}")]
    UnexpectedError(String),
}

#[cfg(feature = "cli")]
pub trait FileFormatOptions: ValueEnum {
    fn format_error(ext: &str) -> SkywayError;
    fn parse(cli_format: Option<Self>, file_path: &Option<PathBuf>) -> Result<Self, SkywayError> {
        if let Some(format) = cli_format {
            Ok(format)
        } else {
            let path = file_path.as_ref().ok_or(Self::format_error(
                "no file path given or format specified.",
            ))?;

            let extension = path.extension().ok_or(Self::format_error(
                format!(
                    "unable to extract extension from path \"{}\"",
                    path.display()
                )
                .as_str(),
            ))?;

            let ext_str = extension.to_str().ok_or(Self::format_error(
                format!(
                    "extension found but could not be converted to a string in path \"{}\"",
                    path.display()
                )
                .as_str(),
            ))?;

            Self::from_str(ext_str, true).map_err(|_| {
                Self::format_error(format!("File extension not recognized: {}", ext_str).as_str())
            })
        }
    }
}

// build a file conversion
// this is the main API for skyway
pub struct ConversionBuilder {
    input_format: InputFileFormat,
    source: Option<PathBuf>,
    #[cfg(feature = "filter")]
    filters: Vec<Box<dyn ElementFilter>>,
    chunk_size: Option<usize>,
}

impl ConversionBuilder {
    pub fn new(input_format: InputFileFormat) -> Self {
        ConversionBuilder {
            input_format,
            source: None,
            #[cfg(feature = "filter")]
            filters: Vec::new(),
            chunk_size: None,
        }
    }

    pub fn with_source(mut self, source: Option<PathBuf>) -> Self {
        self.source = source;
        self
    }

    pub fn with_chunk_size(mut self, chunk_size: usize) -> Self {
        self.chunk_size = Some(chunk_size);
        self
    }

    #[cfg(feature = "filter")]
    pub fn add_filter(mut self, filter: Box<dyn ElementFilter>) -> Self {
        self.filters.push(filter);
        self
    }

    pub fn run_conversion(
        self,
        output_format: OutputFileFormat,
        dest: Option<PathBuf>,
    ) -> Result<(), SkywayError> {
        // set chunk_size, defaulting to 8000,
        // warning if user used custom value with PBF reader
        let chunk_size = if let Some(cs) = self.chunk_size {
            #[cfg(feature = "pbf")]
            if matches!(self.input_format, InputFileFormat::Pbf) {
                warn!("Custom chunk size set, but the PBF does not support custom chunk sizes.");
            }
            cs
        } else {
            8000
        };

        match self.input_format {
            #[cfg(feature = "json")]
            InputFileFormat::Json => JsonReader {}.run_conversion(
                self.source,
                chunk_size,
                #[cfg(feature = "filter")]
                self.filters,
                output_format,
                dest,
            ),
            #[cfg(feature = "opl")]
            InputFileFormat::Opl => OplReader {}.run_conversion(
                self.source,
                chunk_size,
                #[cfg(feature = "filter")]
                self.filters,
                output_format,
                dest,
            ),
            #[cfg(feature = "pbf")]
            InputFileFormat::Pbf => PbfReader {}.run_conversion(
                self.source,
                chunk_size,
                #[cfg(feature = "filter")]
                self.filters,
                output_format,
                dest,
            ),
            #[cfg(feature = "xml")]
            InputFileFormat::Xml => XmlReader {}.run_conversion(
                self.source,
                chunk_size,
                #[cfg(feature = "filter")]
                self.filters,
                output_format,
                dest,
            ),
        }
    }
}
