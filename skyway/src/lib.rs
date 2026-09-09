use thiserror::Error;

#[cfg(feature = "cli")]
use clap::ValueEnum;

#[cfg(any(feature = "pbf", feature = "geojson"))]
use log::warn;

use std::path::PathBuf;

pub mod chunks;
pub mod elements;
mod file_format;
pub mod plan;
pub mod readers;
pub mod runners;
pub mod sort;
pub mod writers;

pub use file_format::OsmFormat;

use plan::{
    ConversionRequest, Order, OrderAssertion, OutputOrderRequest, PipelinePlan, PlanError,
    ReferencePolicy, ResourcePolicy, SourceFacts, TransformFacts, WriterRequirements, build_plan,
};
use readers::*;
use runners::{ParallelOptions, PipelineOutput, run_pipeline};
use sort::SortStrategy;
use writers::*;

// selective imports that deal with filters
#[cfg(feature = "filter")]
pub mod filter;
#[cfg(feature = "filter")]
use filter::{ElementFilter, transform_facts};

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
    #[error("Cannot preserve references: {0}")]
    UnsupportedHistoryInput(String),
    #[error("Invalid filter file: {0}")]
    InvalidFilterFile(String),
    #[error("Cannot parse filter file: {0}")]
    UnparsableFilter(String),
    #[error("Cannot plan conversion: {0}")]
    Plan(#[from] PlanError),
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
    assumed_input_order: Option<Order>,
    allow_temp_files: bool,
    threads: Option<usize>,
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
            assumed_input_order: None,
            allow_temp_files: true,
            threads: None,
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

    /// Assert that the input is already in `order`. This is trusted without
    /// checking; it lets the planner skip sorting when the output format
    /// requires an order the input already has.
    pub fn with_assumed_input_order(mut self, order: Option<Order>) -> Self {
        self.assumed_input_order = order;
        self
    }

    /// Whether temporary files may be used, for example to replay input read
    /// from standard input when preserving filter references.
    pub fn with_allow_temp_files(mut self, allow_temp_files: bool) -> Self {
        self.allow_temp_files = allow_temp_files;
        self
    }

    /// Number of worker threads. `None` uses one per CPU.
    pub fn with_threads(mut self, threads: Option<usize>) -> Self {
        self.threads = threads;
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

    fn parallel_options(&self) -> ParallelOptions {
        ParallelOptions {
            workers: self.threads,
            ..ParallelOptions::default()
        }
    }

    fn request(&self) -> ConversionRequest {
        let order = match self.sort_strategy {
            Some(strategy) => {
                #[cfg(feature = "geojson")]
                if matches!(self.output_format, OsmFormat::GeoJson) {
                    warn!("Sorry, skyway does not support sorting geometric outputs at this time.");
                    OutputOrderRequest::Auto
                } else {
                    strategy.into()
                }
                #[cfg(not(feature = "geojson"))]
                strategy.into()
            }
            None => OutputOrderRequest::Auto,
        };

        #[cfg(feature = "filter")]
        let references = if self.omit_references {
            ReferencePolicy::Omit
        } else {
            ReferencePolicy::Preserve
        };
        #[cfg(not(feature = "filter"))]
        let references = ReferencePolicy::Preserve;

        ConversionRequest { order, references }
    }

    fn source_facts(&self) -> SourceFacts {
        // A local file can be read again. Overpass responses are saved to a
        // temporary file before reading, so they can be too. Standard input
        // cannot.
        #[allow(unused_mut)]
        let mut replayable = self.source.is_some();
        #[cfg(feature = "overpass-queries")]
        if matches!(self.input_format, OsmFormat::OverpassQuery) {
            replayable = true;
        }

        SourceFacts {
            order: self
                .assumed_input_order
                .map(OrderAssertion::asserted_by_user),
            replayable,
        }
    }

    fn transform_facts(&self) -> TransformFacts {
        #[cfg(feature = "filter")]
        {
            transform_facts(&self.filters)
        }
        #[cfg(not(feature = "filter"))]
        {
            TransformFacts::none()
        }
    }

    fn writer_requirements(&self) -> WriterRequirements {
        #[allow(unreachable_patterns)]
        let order = match self.output_format {
            // o5m files are expected to be grouped by type and sorted by ID
            #[cfg(feature = "o5m")]
            OsmFormat::O5m => Some(Order::TypeAndId),
            _ => None,
        };

        WriterRequirements { order }
    }

    /// Decide how this conversion would run, without running it.
    pub fn plan(&self) -> Result<PipelinePlan, SkywayError> {
        // confirm that we can convert between these formats
        OsmFormat::validate_conversion(&self.input_format, &self.output_format)?;

        Ok(build_plan(
            &self.request(),
            &self.source_facts(),
            &self.transform_facts(),
            &self.writer_requirements(),
            &ResourcePolicy {
                allow_temp_files: self.allow_temp_files,
            },
        )?)
    }

    /// Explain the plan for this conversion and how it would be executed.
    pub fn explain_plan(&self) -> Result<String, SkywayError> {
        let plan = self.plan()?;
        let execution = self.parallel_options().explain(&plan);
        Ok(format!("{plan}\n{execution}\n"))
    }

    pub fn run_conversion(self) -> Result<(), SkywayError> {
        let plan = self.plan()?;
        let options = self.parallel_options();
        let preserve_generator = self.preserve_generator;

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

        // An Overpass response is kept on disk until the pipeline is done
        // with it, since a plan may read it more than once.
        #[cfg(feature = "overpass-queries")]
        let mut overpass_temp_file: Option<NamedTempFile> = None;

        let PipelineOutput {
            elements,
            metadata,
            worker,
        } = match self.input_format {
            #[cfg(feature = "json")]
            OsmFormat::Json => run_pipeline(
                JsonReader {},
                self.source,
                chunk_size,
                #[cfg(feature = "filter")]
                self.filters,
                plan,
                preserve_generator,
                options,
            )?,
            #[cfg(feature = "opl")]
            OsmFormat::Opl => run_pipeline(
                OplReader {},
                self.source,
                chunk_size,
                #[cfg(feature = "filter")]
                self.filters,
                plan,
                preserve_generator,
                options,
            )?,
            #[cfg(feature = "overpass-queries")]
            OsmFormat::OverpassQuery => {
                let temp_file = NamedTempFile::new().map_err(|e| {
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
                    temp_file.path(),
                )?;

                let temp_path = temp_file.path().to_path_buf();
                overpass_temp_file = Some(temp_file);

                // Note that rebuild geometry defaults to `true` when OverpassQuery is the input format
                match output_format {
                    OverpassOutputFormat::Json => {
                        unimplemented!()
                        // run_pipeline(
                        //     JsonReader::new(self.rebuild_geometry.unwrap_or(true)),
                        //     Some(temp_path),
                        //     chunk_size,
                        //     #[cfg(feature = "filter")]
                        //     self.filters,
                        //     plan,
                        //     preserve_generator,
                        //     options,
                        //)?
                    }
                    OverpassOutputFormat::Xml => run_pipeline(
                        XmlReader::new(self.rebuild_geometry.unwrap_or(true)),
                        Some(temp_path),
                        chunk_size,
                        #[cfg(feature = "filter")]
                        self.filters,
                        plan,
                        preserve_generator,
                        options,
                    )?,
                }
            }
            #[cfg(feature = "pbf")]
            OsmFormat::Pbf => run_pipeline(
                PbfReader {},
                self.source,
                chunk_size,
                #[cfg(feature = "filter")]
                self.filters,
                plan,
                preserve_generator,
                options,
            )?,
            #[cfg(feature = "xml")]
            OsmFormat::Xml => run_pipeline(
                XmlReader::new(self.rebuild_geometry.unwrap_or(false)),
                self.source,
                chunk_size,
                #[cfg(feature = "filter")]
                self.filters,
                plan,
                preserve_generator,
                options,
            )?,
            _ => unreachable!(), // have already checked the validity of input and output
        };

        #[allow(unreachable_patterns)]
        let write_result: Result<(), SkywayError> = match self.output_format {
            #[cfg(feature = "geojson")]
            OsmFormat::GeoJson => GeoJsonWriter {}.write(elements, metadata, self.dest),
            #[cfg(feature = "json")]
            OsmFormat::Json => JsonWriter { overpass: false }.write(elements, metadata, self.dest),
            #[cfg(feature = "o5m")]
            OsmFormat::O5m => O5mWriter {}.write(elements, metadata, self.dest),
            #[cfg(feature = "opl")]
            OsmFormat::Opl => OplWriter {}.write(elements, metadata, self.dest),
            #[cfg(feature = "xml")]
            OsmFormat::Xml => XmlWriter {}.write(elements, metadata, self.dest),
            _ => Err(SkywayError::UnexpectedError(
                "A file conversion was attempted with an unknown output format.".to_owned(),
            )),
        };

        // The writer stops reading when it fails, which also stops the
        // pipeline; report the writer's error first in that case.
        let pipeline_result = worker.finish();

        #[cfg(feature = "overpass-queries")]
        drop(overpass_temp_file);

        write_result.and(pipeline_result)
    }
}
