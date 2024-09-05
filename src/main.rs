use clap::Parser;
use log::{error, info};
use std::fs;
use std::io::{stdin, stdout};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::mpsc;
use std::thread;

use skyway::elements::{Element, Metadata};
use skyway::{
    filter_elements, read_file, write_file, InputFileFormat, OutputFileFormat, SkywayError,
};

fn get_file_extension(path: &Option<String>) -> Option<&str> {
    path.as_ref()
        .and_then(|p| std::path::Path::new(p).extension())
        .and_then(|ext| ext.to_str())
}

fn parse_format<T: FromStr>(
    cli_format: &Option<String>,
    file_path: &Option<String>,
    io_error: SkywayError,
) -> Result<T, SkywayError>
where
    T::Err: std::fmt::Display,
{
    if let Some(format) = cli_format {
        T::from_str(format).map_err(|_| {
            error!("Could not parse file format: {}", format);
            io_error
        })
    } else {
        match get_file_extension(file_path) {
            Some(ext) => T::from_str(ext).map_err(|_| {
                error!("File extension not recognized: {}", ext);
                io_error
            }),
            None => {
                error!("No file format specified.");
                Err(io_error)
            }
        }
    }
}

#[derive(Parser)]
#[command(name = "skyway")]
#[command(author = "Jacob Hall <email@jacobhall.net>")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Converts OpenStreetMap data between various file formats")]
struct Cli {
    // Path to filter file
    #[arg(long)]
    filter: Option<Vec<String>>,

    // Source file format
    #[arg(long)]
    from: Option<String>,

    // Destination file format
    #[arg(long)]
    to: Option<String>,

    // Path to input file
    #[arg(long)]
    input: Option<String>,

    // Path to output file
    #[arg(long)]
    output: Option<String>,
}

fn main() -> Result<(), SkywayError> {
    env_logger::init();

    let cli = Cli::parse();

    let from =
        parse_format::<InputFileFormat>(&cli.from, &cli.input, SkywayError::UnknownInputFormat)?;
    info!("Input format determined: {:?}", from);

    let to =
        parse_format::<OutputFileFormat>(&cli.to, &cli.output, SkywayError::UnknownOutputFormat)?;
    info!("Output format determined: {:?}", to);

    // will hold this document's metadata
    #[allow(clippy::needless_late_init)]
    let metadata: Metadata;

    // channel for sending elements from the reader to either
    // a) the filter or b) the writer (if not using a filter)
    let (reader_sender, reader_reciever) = mpsc::channel();
    let (metadata_sender, metadata_reciever) = mpsc::channel();

    // spawn a thread that reads the file and spits OSM element
    // data into the channel, to be passed into the filter
    // or data writer
    let read_thread = thread::spawn(move || match cli.input {
        None => read_file(reader_sender, metadata_sender, from, stdin()),
        Some(a) => match fs::File::open(PathBuf::from(a)) {
            Ok(b) => read_file(reader_sender, metadata_sender, from, b),
            Err(e) => {
                panic!("Unable to open input file: {e:?}");
            }
        },
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
    let mut this_sender: mpsc::Sender<Element>;
    let mut last_reciever: mpsc::Receiver<Element> = reader_reciever;
    let mut next_receiver: mpsc::Receiver<Element>;

    // if there are any filters
    if let Some(filters) = cli.filter {
        // for each of the filters (could be one, or more)
        for filter in filters {
            let filter_contents = match fs::read_to_string(filter) {
                Ok(v) => v,
                Err(e) => {
                    panic!("Unable to read filter file: {e:?}");
                }
            };
            (this_sender, next_receiver) = mpsc::channel();
            filter_threads.push(Some(thread::spawn(move || {
                filter_elements(filter_contents.as_str(), last_reciever, this_sender);
            })));
            last_reciever = next_receiver;
        }
    }

    let write_thread = thread::spawn(move || match cli.output {
        None => write_file(last_reciever, metadata, to, stdout()),
        Some(a) => match fs::File::create(PathBuf::from(a)) {
            Ok(b) => write_file(last_reciever, metadata, to, b),
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
