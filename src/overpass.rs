use crate::{SkywayError, readers::get_reader};
use std::path::{Path, PathBuf};

pub fn query_endpoint(
    src: Option<PathBuf>,
    endpoint: &str,
    dest: &Path, // this is a tempfile
) -> Result<(), SkywayError> {
    // Read query to String
    let mut query = String::new();
    get_reader(src).read_to_string(&mut query).map_err(|e| {
        SkywayError::InvalidInputFile(format!("Unable to read input file to String: {}", e))
    })?;

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

    Ok(())
}
