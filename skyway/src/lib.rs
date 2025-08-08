use chunks::ElementChunk;
use elements::Metadata;
use sort::SortStrategy;
use thiserror::Error;

#[cfg(feature = "cli")]
use clap::ValueEnum;

#[cfg(feature = "pbf")]
use log::warn;

use std::{path::PathBuf, sync::mpsc::Receiver};

pub mod chunks;
pub mod elements;
mod file_format;
pub mod readers;
pub mod sort;
pub mod writers;

pub use file_format::OsmFormat;

use readers::*;
use writers::*;

// selective imports that deal with filters
#[cfg(feature = "filter")]
pub mod filter;
#[cfg(feature = "filter")]
use filter::ElementFilter;

#[cfg(feature = "overpass-queries")]
mod overpass;
#[cfg(feature = "overpass-queries")]
use overpass::{OverpassOutputFormat, query_endpoint};
#[cfg(feature = "overpass-queries")]
use tempfile::NamedTempFile;

/// Errors skyway might return.
#[derive(Error, Debug)]
pub enum SkywayError {
    #[error("Cannot determine file format: {0}")]
    UnknownFormat(String),
    #[error("Cannot perform read operation: {0}")]
    UnsupportedRead(String),
    #[error("Cannot perform write operation: {0}")]
    UnsupportedWrite(String),
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("File already exists")]
    OutputFileExists,
    #[error("Invalid input file: {0}")]
    InvalidInputFile(String),
    #[error("Invalid filter file: {0}")]
    InvalidFilterFile(String),
    #[error("Cannot parse filter file: {0}")]
    UnparsableFilter(String),
    #[error("Unexpected error (this is a bug): {0}")]
    UnexpectedError(String),
}

/// Validate input path, taking into account user's overwrite preference
pub fn validate_input_with_overwrite_check(
    input: Option<PathBuf>,
    output: Option<PathBuf>,
    no_overwrite: bool,
) -> Result<Option<PathBuf>, SkywayError> {
    match input {
        Some(path) => match no_overwrite && output.is_some_and(|p| p.exists()) {
            true => Err(SkywayError::OutputFileExists),
            false => Ok(Some(path)),
        },
        None => Ok(None),
    }
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

/// Builder for file conversions
// this is the main API for skyway
pub struct ConversionBuilder {
    input_format: OsmFormat,
    output_format: OsmFormat,
    source: Option<PathBuf>,
    dest: Option<PathBuf>,
    #[cfg(feature = "filter")]
    filters: Vec<Box<dyn ElementFilter>>,
    #[cfg(feature = "filter")]
    omit_references: bool,
    #[cfg(feature = "overpass-queries")]
    endpoint: Option<String>,
    sort: bool,
    sort_strategy: Option<SortStrategy>,
    chunk_size: Option<usize>,
    preserve_generator: bool,
    rebuild_geometry: Option<bool>,
}

impl ConversionBuilder {
    pub fn new(input_format: OsmFormat, output_format: OsmFormat) -> Self {
        ConversionBuilder {
            input_format,
            output_format,
            source: None,
            dest: None,
            #[cfg(feature = "filter")]
            filters: Vec::new(),
            #[cfg(feature = "filter")]
            omit_references: false,
            #[cfg(feature = "overpass-queries")]
            endpoint: None,
            sort: false,
            sort_strategy: None,
            chunk_size: None,
            preserve_generator: true,
            rebuild_geometry: None,
        }
    }

    // TODO: I don't love some of these with_xxx functions. These could just be
    // public fields in the struct. I wanted to use the builder pattern here but
    // it might not make sense.

    pub fn with_dest(mut self, dest: Option<PathBuf>) -> Self {
        self.dest = dest;
        self
    }

    #[cfg(feature = "overpass-queries")]
    pub fn with_endpoint(mut self, endpoint: String) -> Self {
        self.endpoint = Some(endpoint);
        self
    }

    #[cfg(feature = "filter")]
    pub fn with_omit_references(mut self, omit_references: bool) -> Self {
        self.omit_references = omit_references;
        self
    }

    pub fn with_preserve_generator(mut self, preserve_generator: bool) -> Self {
        self.preserve_generator = preserve_generator;
        self
    }

    pub fn with_rebuild_geometry(mut self, rebuild_geometry: bool) -> Self {
        self.rebuild_geometry = Some(rebuild_geometry);
        self
    }

    pub fn with_sort(mut self, sort: bool) -> Self {
        self.sort = sort;
        self
    }

    pub fn with_sort_strategy(mut self, sort_strategy: Option<SortStrategy>) -> Self {
        self.sort_strategy = sort_strategy;
        self
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

    pub fn run_conversion(self) -> Result<(), SkywayError> {
        // confirm that we can convert between these formats
        OsmFormat::validate_conversion(&self.input_format, &self.output_format)?;

        // set chunk_size, defaulting to 8000,
        // warning if user used custom value with PBF reader
        let chunk_size = if let Some(cs) = self.chunk_size {
            #[cfg(feature = "pbf")]
            if matches!(self.input_format, OsmFormat::Pbf) {
                warn!(
                    "Custom chunk size set, but the PBF writer does not support custom chunk sizes."
                );
            }
            cs
        } else {
            8000
        };

        let sort_strategy = match self.sort_strategy {
            Some(s) => {
                // it's a big deal if a non-standard sort strategy is used for 05m,
                // so let's warn the user in that case
                #[cfg(feature = "o5m")]
                if matches!(&self.output_format, OsmFormat::O5m)
                    && !matches!(s, SortStrategy::TypeAndId)
                {
                    warn!(
                        "You selected a non-standard sort strategy for the o5m format. The output may not be readable by other tools."
                    )
                }

                #[cfg(feature = "geojson")]
                if matches!(&self.output_format, OsmFormat::GeoJson) {
                    warn!("Sorry, skyway does not support sorting geometric outputs at this time.");
                    SortStrategy::None
                } else {
                    s // return what the user requested
                }
                #[cfg(not(feature = "geojson"))]
                s // return what the user requested
            }
            None => match &self.output_format {
                // as above, o5m should be using the TypeAndId sort strategy
                #[cfg(feature = "o5m")]
                OsmFormat::O5m => SortStrategy::TypeAndId,
                // otherwise we do not sort by default
                _ => SortStrategy::None,
            },
        };

        let (element_chunk_receiver, metadata_receiver): (
            Receiver<ElementChunk>,
            Receiver<Metadata>,
        ) = match self.input_format {
            #[cfg(feature = "json")]
            OsmFormat::Json => JsonReader {}.run_conversion(
                self.source,
                chunk_size,
                #[cfg(feature = "filter")]
                self.filters,
                #[cfg(feature = "filter")]
                self.omit_references,
                sort_strategy,
                self.preserve_generator,
            )?,
            #[cfg(feature = "opl")]
            OsmFormat::Opl => OplReader {}.run_conversion(
                self.source,
                chunk_size,
                #[cfg(feature = "filter")]
                self.filters,
                #[cfg(feature = "filter")]
                self.omit_references,
                sort_strategy,
                self.preserve_generator,
            )?,
            #[cfg(feature = "overpass-queries")]
            OsmFormat::OverpassQuery => {
                let overpass_temp_file = NamedTempFile::new().map_err(|e| {
                    SkywayError::UnexpectedError(format!(
                        "Unable to create tempfile for Overpass endpoint response: {}",
                        e
                    ))
                })?;

                let output_format = query_endpoint(
                    self.source,
                    &self
                        .endpoint
                        .unwrap_or("https://overpass-api.de/api/interpreter".to_string()),
                    overpass_temp_file.path(),
                )?;

                // Note that rebuild geometry defaults to `true` when OverpassQuery is the input format
                match output_format {
                    OverpassOutputFormat::Json => {
                        unimplemented!()
                        // JsonReader::new(self.rebuild_geometry.unwrap_or(true))
                        // .run_conversion(
                        //     Some(overpass_temp_file.path().to_path_buf()),
                        //     chunk_size,
                        //     #[cfg(feature = "filter")]
                        //     self.filters,
                        //     #[cfg(feature = "filter")]
                        //     self.omit_references,
                        //     sort_strategy,
                        //     self.preserve_generator,
                        //)?)
                    }
                    OverpassOutputFormat::Xml => {
                        XmlReader::new(self.rebuild_geometry.unwrap_or(true)).run_conversion(
                            Some(overpass_temp_file.path().to_path_buf()),
                            chunk_size,
                            #[cfg(feature = "filter")]
                            self.filters,
                            #[cfg(feature = "filter")]
                            self.omit_references,
                            sort_strategy,
                            self.preserve_generator,
                        )?
                    }
                }
            }
            #[cfg(feature = "pbf")]
            OsmFormat::Pbf => PbfReader {}.run_conversion(
                self.source,
                chunk_size,
                #[cfg(feature = "filter")]
                self.filters,
                #[cfg(feature = "filter")]
                self.omit_references,
                sort_strategy,
                self.preserve_generator,
            )?,
            #[cfg(feature = "xml")]
            OsmFormat::Xml => XmlReader::new(self.rebuild_geometry.unwrap_or(false))
                .run_conversion(
                    self.source,
                    chunk_size,
                    #[cfg(feature = "filter")]
                    self.filters,
                    #[cfg(feature = "filter")]
                    self.omit_references,
                    sort_strategy,
                    self.preserve_generator,
                )?,
            _ => unreachable!(), // have already checked the validity of input and output
        };

        #[allow(unreachable_patterns)]
        match self.output_format {
            #[cfg(feature = "geojson")]
            OsmFormat::GeoJson => {
                GeoJsonWriter {}.write(element_chunk_receiver, metadata_receiver, self.dest)
            }
            #[cfg(feature = "json")]
            OsmFormat::Json => JsonWriter { overpass: false }.write(
                element_chunk_receiver,
                metadata_receiver,
                self.dest,
            ),
            #[cfg(feature = "o5m")]
            OsmFormat::O5m => {
                O5mWriter {}.write(element_chunk_receiver, metadata_receiver, self.dest)
            }
            #[cfg(feature = "opl")]
            OsmFormat::Opl => {
                OplWriter {}.write(element_chunk_receiver, metadata_receiver, self.dest)
            }
            #[cfg(feature = "xml")]
            OsmFormat::Xml => {
                XmlWriter {}.write(element_chunk_receiver, metadata_receiver, self.dest)
            }
            _ => Err(SkywayError::UnexpectedError(
                "A file conversion was attempted with an unknown output format.".to_owned(),
            )),
        }
    }
}
