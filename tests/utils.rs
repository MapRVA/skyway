use serde::Deserialize;
use skyway::SkywayError;
use skyway::filter::auto_parse_filter;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use toml;

use skyway::{
    OsmFormat,
    chunks::ElementChunk,
    elements::{Element, Metadata},
    filter::ElementFilter,
    readers::Reader,
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
    pub expected_output: String,
}

pub fn read_elements_and_metadata(
    path: &Path,
    filters: Vec<Box<dyn ElementFilter>>,
    omit_references: bool,
    preserve_generator: bool,
) -> Result<(Receiver<ElementChunk>, Receiver<Metadata>), SkywayError> {
    let chunk_size = 8000;
    let sort_strategy = SortStrategy::None;

    match OsmFormat::parse(None, &Some(path.to_owned())) {
        #[cfg(feature = "opl")]
        Ok(OsmFormat::Opl) => OplReader {}.run_conversion(
            Some(path.to_owned()),
            chunk_size,
            filters,
            omit_references,
            sort_strategy,
            preserve_generator,
        ),
        #[cfg(feature = "xml")]
        Ok(OsmFormat::Xml) => XmlReader::new(false).run_conversion(
            Some(path.to_owned()),
            chunk_size,
            filters,
            omit_references,
            sort_strategy,
            preserve_generator,
        ),
        #[cfg(feature = "json")]
        Ok(OsmFormat::Json) => JsonReader {}.run_conversion(
            Some(path.to_owned()),
            chunk_size,
            filters,
            omit_references,
            sort_strategy,
            preserve_generator,
        ),
        #[cfg(feature = "pbf")]
        Ok(OsmFormat::Pbf) => PbfReader {}.run_conversion(
            Some(path.to_owned()),
            chunk_size,
            filters,
            omit_references,
            sort_strategy,
            preserve_generator,
        ),
        _ => Err(SkywayError::UnexpectedError(
            "Unable to parse input file type.".to_string(),
        )),
    }
}

pub fn run_test(conversion_dir: &Path) {
    let config_path = conversion_dir.join("config.toml");
    let config_content = std::fs::read_to_string(config_path).unwrap();
    let config: ConversionConfig = toml::from_str(&config_content).unwrap();

    let input_path = conversion_dir.join(&config.input);
    let expected_output_path = conversion_dir.join(&config.expected_output);
    let omit_references = match config.omit_references {
        Some(b) => b,
        None => false,
    };

    let preserve_generator = match config.preserve_generator {
        Some(p) => p,
        None => false,
    };

    let mut parsed_filters = Vec::new();
    if let Some(filters) = config.filters {
        for filter_contents in filters {
            let filter_path = conversion_dir.join(&filter_contents);
            let actual_filter_content = std::fs::read_to_string(filter_path).unwrap();
            let filter = auto_parse_filter(&actual_filter_content).unwrap();
            parsed_filters.push(filter);
        }
    }

    // Run the input data through the reader.
    let (output_element_receiver, _) = read_elements_and_metadata(
        &input_path,
        parsed_filters,
        omit_references,
        preserve_generator,
    )
    .unwrap();
    let output_chunks: Vec<ElementChunk> = output_element_receiver.iter().collect();
    let mut output_elements: Vec<Element> = Vec::new();
    for chunk in output_chunks {
        output_elements.extend(chunk.into_iter());
    }

    // Read the elements from the expected output file.
    let (expected_element_receiver, _) =
        read_elements_and_metadata(&expected_output_path, Vec::new(), false, true).unwrap();
    let expected_chunks: Vec<ElementChunk> = expected_element_receiver.iter().collect();
    let mut expected_elements: Vec<Element> = Vec::new();
    for chunk in expected_chunks {
        expected_elements.extend(chunk.into_iter());
    }

    // Make sure number of elements in output vs. expected match.
    assert_eq!(output_elements.len(), expected_elements.len());

    // Make sure each element in output vs. expected are the same.
    for (_, (actual, expected)) in output_elements
        .iter()
        .zip(expected_elements.iter())
        .enumerate()
    {
        assert_eq!(actual, expected);
    }
}

pub fn setup_test(path_dirs: &[&str], test_name: &str) {
    let mut test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    for dir in path_dirs {
        test_dir = test_dir.join(dir);
    }
    test_dir = test_dir.join(test_name);

    run_test(&test_dir);
}

#[macro_export]
macro_rules! define_test {
    ($test_name:ident) => {
        #[test]
        fn $test_name() {
            $crate::utils::setup_test(CURRENT_DIR, stringify!($test_name));
        }
    };
}
