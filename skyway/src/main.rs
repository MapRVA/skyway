use clap::Parser;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use log::info;

use std::{path::PathBuf, process};

use skyway::{
    ConversionBuilder, OsmFormat, SkywayError, sort::SortStrategy,
    validate_input_with_overwrite_check,
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
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(100));
            progress_clone.tick();
            if progress_clone.is_finished() {
                break;
            }
        }
    });
    progress
}

#[derive(Parser)]
#[command(name = "skyway")]
#[command(author = "Jacob Hall <email@jacobhall.net>")]
#[command(version)]
#[command(about = "Converts OpenStreetMap data between various file formats")]
struct Cli {
    /// Source file format
    #[arg(long)]
    from: Option<String>,

    /// Destination file format
    #[arg(long)]
    to: Option<String>,

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

    /// Sort order for output elements
    #[arg(long)]
    #[arg(value_parser = clap::value_parser!(SortStrategy))]
    sort_strategy: Option<SortStrategy>,

    #[cfg(feature = "overpass-queries")]
    /// Endpoint for Overpass API server (only used if input format is overpass-query)
    #[arg(long)]
    endpoint: Option<String>,

    /// Do not include referenced elements unless they themselves pass through filters
    #[arg(long)]
    omit_references: bool,

    /// If output file already exists, don't overwrite it
    #[arg(long)]
    no_overwrite: bool,

    /// Do not replace generator metadata in output with skyway info
    #[arg(long)]
    preserve_generator: bool,

    /// Create fake elements to rebuild geometries from Overpass geom output
    #[arg(long)]
    rebuild_geometry: bool,

    /// Maximum number of elements to store in each chunk passed between threads
    #[arg(long)]
    chunk_size: Option<usize>,
}

fn run() -> Result<(), SkywayError> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
        .target(env_logger::Target::Stderr)
        .init();

    let cli = Cli::parse();

    let from = OsmFormat::parse(cli.from, &cli.input)?;
    info!("Input format determined: {:?}", from);

    let to = OsmFormat::parse(cli.to, &cli.output)?;
    info!("Output format determined: {:?}", to);

    let src = validate_input_with_overwrite_check(cli.input, cli.output.clone(), cli.no_overwrite)?;

    // create a ConversionBuilder that will handle the conversion
    let mut conversion_builder = ConversionBuilder::new(from, to)
        .with_source(src)
        .with_dest(cli.output)
        .with_preserve_generator(cli.preserve_generator)
        .with_rebuild_geometry(cli.rebuild_geometry);

    #[cfg(feature = "filter")]
    {
        conversion_builder = conversion_builder.with_omit_references(cli.omit_references);
    }

    #[cfg(feature = "overpass-queries")]
    {
        if let Some(endpoint) = cli.endpoint {
            conversion_builder = conversion_builder.with_endpoint(endpoint);
        }
    }

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

    conversion_builder.run_conversion()?;

    progress.finish_with_message("Running conversion...done");

    Ok(())
}

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err}");
        process::exit(1);
    }
}
