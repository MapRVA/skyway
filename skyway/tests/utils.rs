use serde::Deserialize;
use skyway::SkywayError;
use skyway::filter::auto_parse_filter;
use std::path::{Path, PathBuf};
use toml;

use skyway::{
    OsmFormat,
    chunks::ElementChunk,
    elements::{Element, Metadata},
    filter::{ElementFilter, transform_facts},
    plan::{
        ConversionRequest, Order, OrderAssertion, OutputOrderRequest, PipelinePlan,
        ReferencePolicy, ResourcePolicy, SourceFacts, WriterRequirements, build_plan,
    },
    runners::{ParallelOptions, PipelineOutput, run_pipeline},
    sort::SortStrategy,
};

#[cfg(feature = "json")]
use skyway::readers::JsonReader;
#[cfg(feature = "opl")]
use skyway::readers::OplReader;
#[cfg(feature = "pbf")]
use skyway::readers::PbfReader;
#[cfg(feature = "xml")]
use skyway::readers::XmlReader;

#[derive(Debug, Deserialize)]
pub struct ConversionConfig {
    pub name: String,
    pub description: String,
    pub input: String,
    pub filters: Option<Vec<String>>,
    pub omit_references: Option<bool>,
    pub preserve_generator: Option<bool>,
    /// One of "type", "id", "type-id", or "none".
    pub sort_strategy: Option<String>,
    /// One of "type", "id", or "type-id".
    pub assume_input_order: Option<String>,
    pub expected_output: String,
}

fn parse_order(name: &str) -> Order {
    match name {
        "type" => Order::Type,
        "id" => Order::Id,
        "type-id" => Order::TypeAndId,
        other => panic!("Unknown order in test config: {other}"),
    }
}

fn parse_sort_strategy(name: &str) -> SortStrategy {
    match name {
        "none" => SortStrategy::None,
        "type" => SortStrategy::Type,
        "id" => SortStrategy::Id,
        "type-id" => SortStrategy::TypeAndId,
        other => panic!("Unknown sort strategy in test config: {other}"),
    }
}

/// How a test input should be read.
#[derive(Clone, Copy, Debug, Default)]
pub struct ReadOptions {
    pub omit_references: bool,
    pub preserve_generator: bool,
    pub sort_strategy: Option<SortStrategy>,
    pub assume_input_order: Option<Order>,
    /// Pretend the input cannot be read twice, forcing the spooled replay.
    pub not_replayable: bool,
    /// Elements per chunk. A small value splits a small fixture across chunks,
    /// so that per-chunk work really is merged rather than done in one pass.
    pub chunk_size: Option<usize>,
}

fn build_test_plan(filters: &[Box<dyn ElementFilter>], options: &ReadOptions) -> PipelinePlan {
    let request = ConversionRequest {
        order: options
            .sort_strategy
            .map(OutputOrderRequest::from)
            .unwrap_or(OutputOrderRequest::Auto),
        references: if options.omit_references {
            ReferencePolicy::Omit
        } else {
            ReferencePolicy::Preserve
        },
    };

    let source = SourceFacts {
        order: options
            .assume_input_order
            .map(OrderAssertion::asserted_by_user),
        replayable: !options.not_replayable,
    };

    build_plan(
        &request,
        &source,
        &transform_facts(filters),
        &WriterRequirements::default(),
        &ResourcePolicy::default(),
    )
    .expect("test plan should be valid")
}

pub fn read_elements_and_metadata(
    path: &Path,
    filters: Vec<Box<dyn ElementFilter>>,
    options: &ReadOptions,
) -> Result<PipelineOutput, SkywayError> {
    let chunk_size = options.chunk_size.unwrap_or(8000);
    let plan = build_test_plan(&filters, options);
    let runner_options = ParallelOptions::default();

    match OsmFormat::parse(None, &Some(path.to_owned())) {
        #[cfg(feature = "opl")]
        Ok(OsmFormat::Opl) => run_pipeline(
            OplReader {},
            Some(path.to_owned()),
            chunk_size,
            filters,
            plan,
            options.preserve_generator,
            runner_options,
        ),
        #[cfg(feature = "xml")]
        Ok(OsmFormat::Xml) => run_pipeline(
            XmlReader::new(false),
            Some(path.to_owned()),
            chunk_size,
            filters,
            plan,
            options.preserve_generator,
            runner_options,
        ),
        #[cfg(feature = "json")]
        Ok(OsmFormat::Json) => run_pipeline(
            JsonReader {},
            Some(path.to_owned()),
            chunk_size,
            filters,
            plan,
            options.preserve_generator,
            runner_options,
        ),
        #[cfg(feature = "pbf")]
        Ok(OsmFormat::Pbf) => run_pipeline(
            PbfReader {},
            Some(path.to_owned()),
            chunk_size,
            filters,
            plan,
            options.preserve_generator,
            runner_options,
        ),
        _ => Err(SkywayError::UnexpectedError(
            "Unable to parse input file type.".to_string(),
        )),
    }
}

/// Read every element from `path`, returning them along with the metadata.
pub fn collect_elements(
    path: &Path,
    filters: Vec<Box<dyn ElementFilter>>,
    options: &ReadOptions,
) -> (Vec<Element>, Option<Metadata>) {
    let PipelineOutput {
        elements,
        metadata,
        worker,
    } = read_elements_and_metadata(path, filters, options).unwrap();

    let chunks: Vec<ElementChunk> = elements.iter().collect();
    worker.finish().unwrap();

    let mut all_elements: Vec<Element> = Vec::new();
    for chunk in chunks {
        all_elements.extend(chunk.into_iter());
    }

    (all_elements, metadata.iter().next())
}

/// Parse one filter file from a test directory.
pub fn filter_file(conversion_dir: &Path, name: &str) -> Box<dyn ElementFilter> {
    let contents = std::fs::read_to_string(conversion_dir.join(name)).unwrap();
    auto_parse_filter(&contents).unwrap()
}

fn parse_filters(conversion_dir: &Path, config: &ConversionConfig) -> Vec<Box<dyn ElementFilter>> {
    match &config.filters {
        Some(filters) => filters
            .iter()
            .map(|name| filter_file(conversion_dir, name))
            .collect(),
        None => Vec::new(),
    }
}

/// Run a conversion for its outcome only, discarding the elements it produces.
///
/// [`collect_elements`] unwraps the pipeline result, so it cannot be used for
/// input that is expected to be rejected.
pub fn conversion_result(
    input: &Path,
    filters: Vec<Box<dyn ElementFilter>>,
    options: &ReadOptions,
) -> Result<(), SkywayError> {
    let PipelineOutput {
        elements, worker, ..
    } = read_elements_and_metadata(input, filters, options)?;

    // The pipeline only finishes once nothing is left waiting to send, so the
    // element stream has to be drained before joining.
    elements.iter().for_each(drop);
    worker.finish()
}

fn assert_same_elements(actual: &[Element], expected: &[Element], context: &str) {
    // Make sure number of elements in output vs. expected match.
    assert_eq!(
        actual.len(),
        expected.len(),
        "{context}: element count differs"
    );

    // Make sure each element in output vs. expected are the same.
    for (actual, expected) in actual.iter().zip(expected.iter()) {
        assert_eq!(actual, expected, "{context}");
    }
}

pub fn run_test(conversion_dir: &Path) {
    let config_path = conversion_dir.join("config.toml");
    let config_content = std::fs::read_to_string(config_path).unwrap();
    let config: ConversionConfig = toml::from_str(&config_content).unwrap();

    let input_path = conversion_dir.join(&config.input);
    let expected_output_path = conversion_dir.join(&config.expected_output);

    let options = ReadOptions {
        omit_references: config.omit_references.unwrap_or(false),
        preserve_generator: config.preserve_generator.unwrap_or(false),
        sort_strategy: config.sort_strategy.as_deref().map(parse_sort_strategy),
        assume_input_order: config.assume_input_order.as_deref().map(parse_order),
        not_replayable: false,
        chunk_size: None,
    };

    // Read the elements from the expected output file.
    let (expected_elements, _) = collect_elements(
        &expected_output_path,
        Vec::new(),
        &ReadOptions {
            preserve_generator: true,
            ..ReadOptions::default()
        },
    );

    // Run the input data through the reader.
    let (output_elements, _) = collect_elements(
        &input_path,
        parse_filters(conversion_dir, &config),
        &options,
    );
    assert_same_elements(&output_elements, &expected_elements, &config.name);

    // Reference-preserving filters have a second replay strategy for input
    // that cannot be read twice. Both must produce the same output.
    let preserves_references = config.filters.is_some() && !options.omit_references;
    if preserves_references {
        let spooled = ReadOptions {
            not_replayable: true,
            ..options
        };
        let (spooled_elements, _) = collect_elements(
            &input_path,
            parse_filters(conversion_dir, &config),
            &spooled,
        );
        assert_same_elements(
            &spooled_elements,
            &expected_elements,
            &format!("{} (spooled replay)", config.name),
        );
    }
}

/// Locate a fixture directory under `tests/`.
pub fn test_dir(path_dirs: &[&str], test_name: &str) -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    for path_dir in path_dirs {
        dir = dir.join(path_dir);
    }
    dir.join(test_name)
}

pub fn setup_test(path_dirs: &[&str], test_name: &str) {
    run_test(&test_dir(path_dirs, test_name));
}

#[macro_export]
macro_rules! define_test {
    ($test_name:ident) => {
        #[test]
        fn $test_name() {
            $crate::utils::setup_test(CURRENT_DIR, stringify!($test_name));
        }
    };
    ($test_name:ident, $dir_name:literal) => {
        #[test]
        fn $test_name() {
            $crate::utils::setup_test(CURRENT_DIR, $dir_name);
        }
    };
}
