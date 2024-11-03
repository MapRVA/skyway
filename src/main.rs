use clap::Parser;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use log::{info, warn};
use std::fs;
use std::io::stdout;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

use skyway::{
    chunks::{Chunk, ChunkBuilder},
    elements::Metadata,
    filter::{create_filter, filter_elements, ElementFilter},
    readers::InputFileFormat,
    writers::{write_file, OutputFileFormat},
    FileFormatOptions, SkywayError,
};

#[derive(Parser)]
#[command(name = "skyway")]
#[command(author = "Jacob Hall <email@jacobhall.net>")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Converts OpenStreetMap data between various file formats")]
struct Cli {
    /// Path to filter file
    #[arg(long)]
    filter: Option<Vec<String>>,

    /// Source file format
    #[arg(long)]
    from: Option<InputFileFormat>,

    /// Destination file format
    #[arg(long)]
    to: Option<OutputFileFormat>,

    /// Path to input file
    #[arg(long)]
    #[arg(value_parser = clap::value_parser!(PathBuf))]
    input: Option<PathBuf>,

    /// Path to output file
    #[arg(long)]
    #[arg(value_parser = clap::value_parser!(PathBuf))]
    output: Option<PathBuf>,

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

    // will hold this document's metadata
    #[allow(clippy::needless_late_init)]
    let metadata: Metadata;

    // channel for sending elements from the reader to either
    // a) the filter or b) the writer (if not using a filter)
    let (reader_sender, reader_reciever) = mpsc::channel();
    let (metadata_sender, metadata_reciever) = mpsc::channel();

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

    let mut reader = from.generate_reader(cli.input);

    // spawn a thread that reads the file and spits OSM element
    // data into the channel, to be passed into the filter
    // or data writer
    let read_thread = thread::spawn(move || {
        read_progress.set_message("Reading input...");

        reader.read(ChunkBuilder::new(chunksize), reader_sender, metadata_sender);

        // complete reader progress spinner
        read_progress.finish_with_message("Reading input...done");
    });

    metadata = match metadata_reciever.iter().next() {
        Some(m) => m,
        None => {
            panic!("No metadata received from reader!");
        }
    };

    // stack of filter threads that we'll need to hold open until each
    // is done
    let mut filter_threads = Vec::new();

    // create variables that will hold the Sender and Receiver for the
    // current (last created) filter
    let mut this_sender: mpsc::Sender<Chunk>;
    let mut last_receiver: mpsc::Receiver<Chunk> = reader_reciever;
    let mut next_receiver: mpsc::Receiver<Chunk>;

    let mut filters: Vec<Box<dyn ElementFilter>> = Vec::new();

    if let Some(filter_paths) = cli.filter {
        for filter_path in filter_paths {
            filters.push(create_filter(
                fs::read_to_string(&filter_path)
                    .unwrap_or_else(|e| {
                        panic!("Unable to read filter file {}: {}", filter_path, e);
                    })
                    .as_str(),
            ));
        }
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

    let write_progress = multi.add(ProgressBar::new_spinner());
    write_progress.set_style(spinner_style.clone());

    let write_thread = thread::spawn(move || match cli.output {
        None => write_file(last_receiver, metadata, to, stdout(), write_progress),
        Some(a) => match fs::File::create(PathBuf::from(a)) {
            Ok(b) => write_file(last_receiver, metadata, to, b, write_progress),
            Err(e) => {
                panic!("Unable to open output file: {e:?}");
            }
        },
    });

    read_thread.join().expect("Couldn't join on read thread!!");
    for filter_thread in filter_threads {
        let Some(ft) = filter_thread else { continue };
        ft.join().expect("Couldn't join on filter thread!!");
    }
    write_thread
        .join()
        .expect("Couldn't join on write thread!!");

    Ok(())
}
