use std::sync::mpsc;
use std::thread;
use std::io::{stdin, stdout};
use std::fs;
use clap::Parser;
use std::path::PathBuf;

// use config::load_config;

mod elements;

mod readers;
use readers::read_file;

mod filter;
use filter::filter_elements;

mod writers;
use writers::write_file;

// determine current version of crate

const VERSION: &str = env!("CARGO_PKG_VERSION");

// use writers::write_elements;

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
    from: String,

    // Destination file format
    #[arg(long)]
    to: String,

    // Path to input file
    #[arg(long)]
    input: Option<String>,

    // Path to output file
    #[arg(long)]
    output: Option<String>,
}

fn main() {
    eprintln!("skyway v{}", VERSION);

    let cli = Cli::parse();
    
    // will hold this document's metadata
    let metadata: elements::Metadata;
    
    // channel for sending elements from the reader to either
    // a) the filter or b) the writer (if not using a filter)
    let (reader_sender, reader_reciever) = mpsc::channel();
    let (metadata_sender, metadata_reciever) = mpsc::channel();

    // spawn a thread that reads the file and spits OSM element
    // data into the channel, to be passed into the filter
    // or data writer
    let read_thread = thread::spawn(move || {
        match cli.input {
            None => {
                read_file(reader_sender, metadata_sender, &cli.from, stdin())
            },
            Some(a) => match fs::File::open(PathBuf::from(a)) {
                Ok(b) => {
                    read_file(reader_sender, metadata_sender, &cli.from, b)
                },
                Err(e) => {
                    panic!("Unable to open input file: {e:?}");
                }
            }
        }
    });

    metadata = match metadata_reciever.iter().next() {
        Some(m) => m,
        None => {
            panic!("No metadata received from reader!");
        },
    };

    // stack of filter threads that we'll need to hold open until each
    // is done
    let mut filter_threads = Vec::new();

    // create variables that will hold the Sender and Receiver for the
    // current (last created) filter
    let mut this_sender: mpsc::Sender<elements::Element>;
    let mut last_reciever: mpsc::Receiver<elements::Element> = reader_reciever;
    let mut next_receiver: mpsc::Receiver<elements::Element>;

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

    let write_thread =  thread::spawn(move || {
        match cli.output {
            None => {
                write_file(last_reciever, metadata, &cli.to, stdout())
            },
            Some(a) => match fs::File::create(PathBuf::from(a)) {
                Ok(b) => {
                    write_file(last_reciever, metadata, &cli.to, b)
                },
                Err(e) => {
                    panic!("Unable to open output file: {e:?}");
                }
            }

        }
    });
        

    read_thread.join().expect("Couldn't join on read thread!!");
    for filter_thread in filter_threads {
        let Some(ft) = filter_thread else { continue };
        ft.join().expect("Couldn't join on filter thread!!");
    }
    write_thread.join().expect("Couldn't join on write thread!!");
    
}
