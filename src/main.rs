use clap::Parser;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use log::info;

use std::{path::PathBuf, process};

use skyway::{
    readers::InputFileFormat, writers::OutputFileFormat, ConversionBuilder, FileFormatOptions,
    SkywayError,
};

#[cfg(feature = "filter")]
use skyway::filter::filter_from_path;

fn start_progress(message: &str) -> ProgressBar {
    let multi = MultiProgress::new();
    let spinner_style = ProgressStyle::with_template("{prefix:.bold.dim} {spinner} {wide_msg}")
        .unwrap()
        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ");

    let progress = multi.add(ProgressBar::new_spinner());
    progress.set_style(spinner_style.clone());

    progress.set_message(message.to_owned());

    let progress_clone = progress.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(100));
        progress_clone.tick();
        if progress_clone.is_finished() {
            break;
        }
    });
    progress
}

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

fn run() -> Result<(), SkywayError> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
        .target(env_logger::Target::Stderr)
        .init();

    let cli = Cli::parse();

    let from = InputFileFormat::parse(cli.from, &cli.input)?;
    info!("Input format determined: {:?}", from);

    let to = OutputFileFormat::parse(cli.to, &cli.output)?;
    info!("Output format determined: {:?}", to);

    let src = match cli.input {
        Some(path) => match cli.no_overwrite && path.exists() {
            true => return Err(SkywayError::OutputFileExists),
            false => Some(path),
        },
        None => None,
    };

    // create a ConversionBuilder that will handle the conversion
    let mut conversion_builder = ConversionBuilder::new(from).with_source(src);

    if let Some(chunk_size) = cli.chunk_size {
        conversion_builder = conversion_builder.with_chunk_size(chunk_size)
    }

    // if filters were passed, add them to our ConversionBuilder
    #[cfg(feature = "filter")]
    if let Some(filters) = cli.filter {
        for filter in filters {
            let element_filter = filter_from_path(&filter)?;
            conversion_builder = conversion_builder.add_filter(element_filter)
        }
    }

    let progress = start_progress("Running conversion...");

    // run the conversion with our chosen writer and destination
    conversion_builder.run_conversion(to, cli.output)?;

    progress.finish_with_message("Running conversion...done");

    Ok(())
}

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err}");
        process::exit(1);
    }
}
