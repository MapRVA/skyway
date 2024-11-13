use chunks::ChunkBuilder;
use clap::ValueEnum;
use readers::{Reader, Readers};
use thiserror::Error;
use writers::Writer;

use std::{path::PathBuf, sync::mpsc::channel};

pub mod chunks;
pub mod elements;
pub mod readers;
pub mod writers;

// selective imports that deal with filters
#[cfg(feature = "filter")]
use filter::{build_filter, ElementFilter};

#[cfg(feature = "filter")]
pub mod filter;

#[cfg(not(feature = "filter"))]
use std::convert::identity;

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
}

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
    reader: Readers,
    src: Option<PathBuf>,
    filters: Vec<Box<dyn ElementFilter>>,
    chunk_size: usize,
}

impl ConversionBuilder {
    pub fn new(reader: Readers) -> Self {
        ConversionBuilder {
            reader,
            src: None,
            filters: Vec::new(),
            chunk_size: 8000,
        }
    }

    pub fn with_source(mut self, src: Option<PathBuf>) -> Self {
        self.src = src;
        self
    }

    pub fn with_chunk_size(mut self, chunk_size: usize) -> Self {
        self.chunk_size = chunk_size;
        self
    }

    pub fn add_filter(mut self, filter: Box<dyn ElementFilter>) -> Self {
        self.filters.push(filter);
        self
    }

    pub fn run_conversion(self, writer: Box<dyn Writer>, dest: Option<PathBuf>) {
        let (metadata_sender, metadata_receiver) = channel();

        let chunk_builder = ChunkBuilder::new(self.chunk_size);

        let (final_iterator, write_thread) = writer.write_file(metadata_receiver, dest);

        #[cfg(feature = "filter")]
        let filter = build_filter(self.filters);

        #[cfg(not(feature = "filter"))]
        let filter = identity;

        self.reader.read_file(
            self.src,
            metadata_sender,
            chunk_builder,
            filter,
            write_thread,
            final_iterator,
        );
    }
}
