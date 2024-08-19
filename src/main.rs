use std::sync::mpsc;
use std::thread;
use std::io::stdin;
use std::fs;
use clap::Parser;
use std::str::FromStr;
use std::path::PathBuf;

// use config::load_config;

mod elements;

mod readers;
use readers::read_file;

mod filter;
use filter::parse::{parse_filter, filter_elements};

// use writers::write_elements;

#[derive(Parser)]
#[command(name = "skyway")]
#[command(author = "Jacob Hall <email@jacobhall.net>")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Converts OpenStreetMap data between various formats")]
struct Cli {
    // semantic or routing
    // #[arg(long, default_value = "memgraph")]
    // mode: String,

    // Path to configuration TOML
    #[arg(long)]
    filter: Option<String>,

    #[arg(long)]
    from: String,

    // Path to input file
    #[arg(long)]
    input: Option<String>,

    // Hostname of memgraph database
    #[arg(long, default_value = "localhost")]
    hostname: String,

    // Password for memgraph database
    #[arg(long)]
    password: Option<String>,

    // Port for memgraph database
    #[arg(long, default_value = "7687")]
    port: String,

    // Username for memgraph database
    #[arg(long)]
    username: Option<String>,
}

fn main() {
    let cli = Cli::parse();
    let port: u16 = u16::from_str(cli.port.as_str()).unwrap();
    
    let (reader_sender, reader_reciever) = mpsc::channel();

    // spawn a thread that reads the file and spits OSM element
    // data into the channel, to be passed into the filter
    // or data writer
    let read_thread =  thread::spawn(move || {
        match cli.input {
            None => {
                read_file(reader_sender, &cli.from, stdin())
            },
            Some(a) => match fs::File::open(PathBuf::from(a)) {
                Ok(b) => {
                    read_file(reader_sender, &cli.from, b)
                },
                Err(e) => {
                    panic!("Unable to read input file: {e:?}");
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

    let mut counter = 0;
    for _ in filter_reciever {
        counter = counter + 1;
    }
    println!("Counted {counter} elements coming out of the filter.");
        

    read_thread.join().expect("Couldn't join on thread!!");
    match filter_thread {
        Some(ft) => {
            ft.join().expect("Couldn't join on filter thread!!");
        },
        _ => {}
    }


    // pass element iterator from filter into data writer
    // write_elements(element_iterator)
    
}
