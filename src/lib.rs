use thiserror::Error;

pub mod elements;
pub mod filter;
pub mod readers;
pub mod writers;

pub use filter::filter_elements;
pub use readers::{read_file, InputFileFormat};
pub use writers::{write_file, OutputFileFormat};

#[derive(Error, Debug)]
pub enum SkywayError {
    #[error("Unknown input file format")]
    UnknownInputFormat,
    #[error("Unknown output file format")]
    UnknownOutputFormat,
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}
