use crate::{SkywayError, readers::get_reader};
use regex::Regex;
use std::path::{Path, PathBuf};

pub enum OverpassOutputFormat {
    Json,
    Xml,
}

pub fn query_endpoint(
    src: Option<PathBuf>,
    endpoint: &str,
    dest: &Path, // this is a tempfile
) -> Result<OverpassOutputFormat, SkywayError> {
    // Read query to String
    let mut query = String::new();
    get_reader(src).read_to_string(&mut query).map_err(|e| {
        SkywayError::InvalidInputFile(format!("Unable to read input file to String: {}", e))
    })?;

    let re = Regex::new(r"^out:[a-z]$").unwrap();

    let output_format = if query.contains("[out:xml]") {
        OverpassOutputFormat::Xml
    } else if query.contains("[out:json]") {
        OverpassOutputFormat::Json
    } else if re.is_match(&query) {
        return Err(SkywayError::InvalidInputFile(
            "Your Overpass query requests an output format that skyway cannot parse. Please request XML or JSON, if possible.".to_string(),
        ));
    } else {
        OverpassOutputFormat::Xml
    };

    let client = reqwest::blocking::Client::new();
    let response = client
        .post(endpoint)
        .body(query)
        .send()
        .map_err(|e| {
            SkywayError::InvalidInputFile(format!(
                "Error querying Overpass endpoint: {}",
                e.to_string()
            ))
        })?
        .text()
        .map_err(|e| {
            SkywayError::UnexpectedError(format!(
                "Unable to convert Overpass endpoint response to a String: {}",
                e
            ))
        })?;

    // Write response to tempfile
    std::fs::write(dest, &response).map_err(|e| SkywayError::InvalidInputFile(e.to_string()))?;

    Ok(output_format)
}
