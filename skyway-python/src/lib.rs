use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use std::path::PathBuf;

use ::skyway::{
    ConversionBuilder, OsmFormat, SkywayError, filter::filter_from_path,
    validate_input_with_overwrite_check,
};

// FIXME: better, Pythonic errors for skyway
fn skyway_error_to_pyerr(err: SkywayError) -> PyErr {
    PyRuntimeError::new_err(format!("{}", err))
}

#[pyfunction]
#[pyo3(signature = (*, from=None, to=None, input=None, filters=None, output=None, sort_strategy=None, endpoint=None, omit_references=false, no_overwrite=false, preserve_generator=false, rebuild_geometry=false, chunk_size=None))]
fn convert(
    from: Option<String>,
    to: Option<String>,
    input: Option<String>,
    filters: Option<Vec<String>>,
    output: Option<String>,
    sort_strategy: Option<String>,
    endpoint: Option<String>,
    omit_references: bool,
    no_overwrite: bool,
    preserve_generator: bool,
    rebuild_geometry: bool,
    chunk_size: Option<usize>,
) -> PyResult<()> {
    // Convert string paths to PathBufs
    let input_path = input.map(PathBuf::from);
    let output_path = output.map(PathBuf::from);

    let from = OsmFormat::parse(from, &input_path).map_err(skyway_error_to_pyerr)?;
    let to = OsmFormat::parse(to, &output_path).map_err(skyway_error_to_pyerr)?;

    let src = validate_input_with_overwrite_check(input_path, output_path.clone(), no_overwrite)
        .map_err(skyway_error_to_pyerr)?;

    // create a ConversionBuilder that will handle the conversion
    let mut conversion_builder = ConversionBuilder::new(from, to)
        .with_source(src)
        .with_dest(output_path)
        .with_preserve_generator(preserve_generator)
        .with_rebuild_geometry(rebuild_geometry);

    conversion_builder = conversion_builder.with_omit_references(omit_references);

    if let Some(endpoint) = endpoint {
        conversion_builder = conversion_builder.with_endpoint(endpoint);
    }

    if let Some(chunk_size) = chunk_size {
        conversion_builder = conversion_builder.with_chunk_size(chunk_size)
    }

    // if filters were passed, add them to our ConversionBuilder
    if let Some(filters) = filters {
        for filter in filters {
            let filter_path = PathBuf::from(filter);
            let element_filter = filter_from_path(&filter_path).map_err(skyway_error_to_pyerr)?;
            conversion_builder = conversion_builder.add_filter(element_filter)
        }
    }

    conversion_builder
        .run_conversion()
        .map_err(|e| skyway_error_to_pyerr(e))
}

#[pymodule]
fn skyway(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(convert, m)?)
}
