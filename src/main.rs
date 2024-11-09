use clap::Parser;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use log::{info, warn};

use std::path::PathBuf;
use std::sync::mpsc;

#[cfg(feature = "filter")]
use skyway::filter::{create_filter, filter_elements, ElementFilter};
use skyway::{
    chunks::ChunkBuilder,
    readers::{InputFileFormat, Reader},
    writers::{OutputFileFormat, Writer},
    FileFormatOptions, SkywayError,
};

#[cfg(feature = "filter")]
fn setup_filter_chain(
    filter_paths: Vec<String>,
    chunksize: usize,
    mut last_receiver: mpsc::Receiver<Chunk>,
    multi: &MultiProgress,
    spinner_style: &ProgressStyle,
) -> (Vec<Option<thread::JoinHandle<()>>>, mpsc::Receiver<Chunk>) {
    // stack of filter threads that we'll need to hold open until each
    // is done
    let mut filter_threads = Vec::new();

    // create variables that will hold the Sender and Receiver for the
    // current (last created) filter
    let mut this_sender: mpsc::Sender<Chunk>;
    let mut next_receiver: mpsc::Receiver<Chunk>;

    let mut filters: Vec<Box<dyn ElementFilter>> = Vec::new();

    for filter_path in filter_paths {
        filters.push(create_filter(
            fs::read_to_string(&filter_path)
                .unwrap_or_else(|e| {
                    panic!("Unable to read filter file {}: {}", filter_path, e);
                })
                .as_str(),
        ));
    }

    for filter in filters {
        let filter_progress = multi.add(ProgressBar::new_spinner());
        filter_progress.set_style(spinner_style.clone());

        (this_sender, next_receiver) = mpsc::channel();
        filter_threads.push(Some(thread::spawn(move || {
            filter_elements(
                filter,
                ChunkBuilder::new(chunksize),
                last_receiver,
                this_sender,
                filter_progress,
            );
        })));
        last_receiver = next_receiver;
    }
    (filter_threads, last_receiver)
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
    filter: Option<Vec<String>>,

    /// Path to output file (if not given, writes to stdout)
    #[arg(long)]
    #[arg(value_parser = clap::value_parser!(PathBuf))]
    output: Option<PathBuf>,

    /// If output file already exists, don't overwrite it
    #[arg(long)]
    no_overwrite: bool,

    /// Maximum number of elements to store in each chunk passed between threads
    #[arg(long)]
    chunksize: Option<usize>,
}

fn main() -> Result<(), SkywayError> {
    env_logger::init();

    let cli = Cli::parse();

    let from = InputFileFormat::parse(cli.from, &cli.input)?;
    info!("Input format determined: {:?}", from);

    let to = OutputFileFormat::parse(cli.to, &cli.output)?;
    info!("Output format determined: {:?}", to);

    let chunksize: usize = match cli.chunksize {
        None => 8000,
        Some(c) => {
            #[cfg(feature = "pbf")]
            if InputFileFormat::Pbf == from {
                warn!("For now, skyway's PBF reader ignores the chunksize argument.")
            }
            c
        }
    };

    let (metadata_sender, metadata_receiver) = mpsc::channel();

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

    let reader = from.generate_reader();

    #[cfg(feature = "filter")]
    let (filter_threads, last_receiver) = match cli.filter {
        Some(filter_paths) => setup_filter_chain(
            filter_paths,
            chunksize,
            last_receiver,
            &multi,
            &spinner_style,
        ),
        None => (vec![], last_receiver),
    };

    // let write_progress = multi.add(ProgressBar::new_spinner());
    // write_progress.set_style(spinner_style.clone());
    //
    let writer = to.generate_writer();

    let (final_iterator, write_thread) = writer.write_file(metadata_receiver, cli.output);

    reader.read_file(
        src,
        metadata_sender,
        ChunkBuilder::new(chunksize),
        write_thread,
        final_iterator,
    );

    Ok(())
}
