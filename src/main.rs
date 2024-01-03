use clap::Parser;
use std::str::FromStr;
use std::path::Path;

// use config::load_config;

mod loaders;
use loaders::load_file;

// use writers::write_elements;

#[derive(Parser)]
#[command(name = "skyway")]
#[command(author = "Jacob Hall <email@jacobhall.net>")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Converts OpenStreetMap data between various formats")]
struct Cli {
    // semantic or routing
    #[arg(long, default_value = "memgraph")]
    mode: String,

    // Path to configuration TOML
    #[arg(long)]
    config: Option<String>,

    // Path to input file
    #[arg(long)]
    input: String,

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

    let port: u16 = u16::from_str(&cli.port).unwrap();

    let input_file_path = Path::new(&cli.input);

    // load config file
    // let config = load_config(&cli.config);

    // get element iterator from input file
    let element_iterator = load_file(input_file_path);

    // pass element iterator into data writer
    // write_elements(element_iterator)
}
