use clap::ValueEnum;
use thiserror::Error;

use std::path::PathBuf;

pub mod chunks;
pub mod elements;
pub mod filter;
pub mod readers;
pub mod writers;

mod threadpools;

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
