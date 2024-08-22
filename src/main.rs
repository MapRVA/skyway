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
use filter::parse::filter_elements;

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
    filter: Option<String>,

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
    
    let (reader_sender, reader_reciever) = mpsc::channel();

    // spawn a thread that reads the file and spits OSM element
    // data into the channel, to be passed into the filter
    // or data writer
    let read_thread = thread::spawn(move || {
        match cli.input {
            None => {
                read_file(reader_sender, &cli.from, stdin())
            },
            Some(a) => match fs::File::open(PathBuf::from(a)) {
                Ok(b) => {
                    read_file(reader_sender, &cli.from, b)
                },
                Err(e) => {
                    panic!("Unable to open input file: {e:?}");
                }
            }
        }
    });


    let (filter_sender, filter_reciever) = mpsc::channel();

    // if the user passed a filter, read and parse it
    let mut filter_thread: Option<thread::JoinHandle<()>> = None;
    if let Some(filter_path) = cli.filter {
        let filter = match fs::read_to_string(filter_path) {
            Ok(v) => v,
            Err(e) => {
                panic!("Unable to read filter file: {e:?}");
            }
        };
        filter_thread =  Some(thread::spawn(move || {
            filter_elements(filter.as_str(), reader_reciever, filter_sender);
        }));
    }

    let metadata = elements::Metadata {
        version: None,
        generator: None,
        copyright: None,
        license: None,
    };

    let write_thread =  thread::spawn(move || {
        match cli.output {
            None => {
                write_file(filter_reciever, metadata, &cli.to, stdout())
            },
            Some(a) => match fs::File::open(PathBuf::from(a)) {
                Ok(b) => {
                    write_file(filter_reciever, metadata, &cli.to, b)
                },
                Err(e) => {
                    panic!("Unable to open output file: {e:?}");
                }
            }

        }
    });
        

    read_thread.join().expect("Couldn't join on read thread!!");
    if let Some(ft) = filter_thread {
        ft.join().expect("Couldn't join on filter thread!!");
    }
    write_thread.join().expect("Couldn't join on write thread!!");
    
}
