use clap::Parser;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use log::{info, warn};

use std::path::PathBuf;

use skyway::{
    readers::InputFileFormat, writers::OutputFileFormat, ConversionBuilder, FileFormatOptions,
    SkywayError,
};

#[cfg(feature = "filter")]
use skyway::filter::filter_from_path;

#[derive(Parser)]
#[command(name = "skyway")]
#[command(author = "Jacob Hall <email@jacobhall.net>")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Converts OpenStreetMap data between various file formats")]
struct Cli {
    /// Source file format
    #[arg(long)]
    from: Option<InputFileFormat>,

    /// Destination file format
    #[arg(long)]
    to: Option<OutputFileFormat>,

    /// Path to input file (if not given, reads from stdin)
    #[arg(long)]
    #[arg(value_parser = clap::value_parser!(PathBuf))]
    input: Option<PathBuf>,

    /// Path to filter file, may be used multiple times
    #[cfg(feature = "filter")]
    #[arg(long)]
    filter: Option<Vec<PathBuf>>,

    /// Path to output file (if not given, writes to stdout)
    #[arg(long)]
    #[arg(value_parser = clap::value_parser!(PathBuf))]
    output: Option<PathBuf>,

    /// If output file already exists, don't overwrite it
    #[arg(long)]
    no_overwrite: bool,

    /// Maximum number of elements to store in each chunk passed between threads
    #[arg(long)]
    chunk_size: Option<usize>,
}

fn main() -> Result<(), SkywayError> {
    env_logger::init();

    let cli = Cli::parse();

    let from = InputFileFormat::parse(cli.from, &cli.input)?;
    info!("Input format determined: {:?}", from);

    let to = OutputFileFormat::parse(cli.to, &cli.output)?;
    info!("Output format determined: {:?}", to);

    let chunk_size: usize = match cli.chunk_size {
        None => 8000,
        Some(c) => {
            #[cfg(feature = "pbf")]
            if InputFileFormat::Pbf == from {
                warn!("For now, skyway's PBF reader ignores the chunksize argument.")
            }
            c
        }
    };

    let multi = MultiProgress::new();
    let spinner_style = ProgressStyle::with_template("{prefix:.bold.dim} {spinner} {wide_msg}")
        .unwrap()
        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ");

    let read_progress = multi.add(ProgressBar::new_spinner());
    read_progress.set_style(spinner_style.clone());

    //  for reader progress
    let progress_clone = read_progress.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(100));
        progress_clone.tick();
        if progress_clone.is_finished() {
            break;
        }
    });

    let src = match cli.input {
        Some(path) => match cli.no_overwrite && path.exists() {
            true => return Err(SkywayError::OutputFileExists),
            false => Some(path),
        },
        None => None,
    };

    // create a Reader and Writer for this conversion
    let reader = from.generate_reader();
    let writer = to.generate_writer();

    // create a ConversionBuilder that will handle the conversion
    let mut conversion_builder = ConversionBuilder::new(reader)
        .with_source(src)
        .with_chunk_size(chunk_size);

    #[cfg(feature = "filter")]
    if let Some(filters) = cli.filter {
        for filter in filters {
            let element_filter = filter_from_path(&filter)?;
            conversion_builder = conversion_builder.add_filter(element_filter)
        }
    }

    // run the conversion with our chosen writer and destination
    conversion_builder.run_conversion(writer, cli.output);

    Ok(())
}
