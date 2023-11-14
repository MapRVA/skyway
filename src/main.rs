use clap::Parser;
use indicatif::ProgressBar;
use osmpbf::{Element, ElementReader, TagIter};
use rsmgclient::{ConnectParams, Connection, QueryParam};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::str::FromStr;
use toml::Table;

#[derive(Parser)]
#[command(name = "osm2memgraph")]
#[command(author = "Jacob Hall <email@jacobhall.net>")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Loads OpenStreetMap data into Memgraph")]
struct Cli {
    // Path to configuration TOML
    #[arg(long, default_value = "config.toml")]
    config: String,

    // Path to input .pbf file
    #[arg(short, long)]
    input: String,

    // Hostname of memgraph database
    #[arg(long, default_value = "localhost")]
    hostname: String,

    // // Password for memgraph database
    // #[arg(long)]
    // password: String,

    // Port for memgraph database
    #[arg(long, default_value = "7687")]
    port: String,

    // Username for memgraph database
    #[arg(long)]
    username: String,
}

fn load_config(config_path: &str) -> toml::map::Map<std::string::String, toml::Value> {
    let mut file = File::open("config.toml").expect("Failed to open file");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Failed to read configuration file");
    let config = contents.parse::<Table>().unwrap();
    return config;
}

struct TagFilter {
    key: String,
    values: Vec<String>,
}

struct FilterTable {
    filters: Vec<TagFilter>,
}

struct NodeInfo {
    id: i64,
    lat: f64,
    lng: f64,
    tags: Vec<(String, String)>,
}

struct EdgeInfo {
    first_node_id: i64,
    second_node_id: i64,
    tags: Vec<(String, String)>,
}

impl FilterTable {
    fn allows(&self, tags: TagIter<'_>) -> bool {
        for (key, value) in tags {
            for tag_filter in &self.filters {
                if tag_filter.key.eq(&key) {
                    if tag_filter.values.contains(&value.to_owned()) {
                        return true;
                    }
                }
            }
        }
        false
    }
}

fn create_filter_table(table: &toml::Table) -> FilterTable {
    let mut filters = Vec::new();
    for (key, values) in table.iter() {
        let mut value_strings = Vec::new();
        for value in values.as_array().unwrap() {
            value_strings.push(value.as_str().unwrap().to_owned());
        }
        filters.push(TagFilter {
            key: key.to_string(),
            values: value_strings,
        });
    }
    FilterTable { filters: filters }
}

fn main() {
    let cli = Cli::parse();

    let port: u16 = u16::from_str(&cli.port).unwrap();

    let config = load_config(&cli.config);

    let include_table = create_filter_table(config["include"].as_table().unwrap());

    let pbf_reader = ElementReader::from_path(cli.input)
        .expect("input should be a path to readable OSM .pbf file");

    let connect_params = ConnectParams {
        host: Some(cli.hostname),
        port: port,
        ..Default::default()
    };

    let mut connection = Connection::connect(&connect_params)
        .expect("connection parameters should describe an available memgraph database connection");

    let mut total_element_count: u32 = 0;
    let mut selected_element_count: u32 = 0;

    let mut nodes: Vec<NodeInfo> = Vec::new();
    let mut edges: Vec<EdgeInfo> = Vec::new();

    println!("Reading data out of file...");
    pbf_reader
        .for_each(|element| {
            match element {
                // TODO: also support Element::Node
                Element::DenseNode(node) => {
                    let mut tags: Vec<(String, String)> = Vec::new();
                    tags.push(("hello".to_string(), "world".to_string()));

                    nodes.push(NodeInfo {
                        id: node.id(),
                        lat: node.lat(),
                        lng: node.lon(),
                        tags: tags,
                    });
                }
                Element::Way(way) => {
                    total_element_count = total_element_count + 1;
                    if include_table.allows(way.tags()) {
                        selected_element_count = selected_element_count + 1;

                        let mut tags: Vec<(String, String)> = Vec::new();
                        tags.push(("hello".to_string(), "world".to_string()));

                        let mut refs = way.refs();

                        let mut first_node_id: i64 = refs.next().unwrap();

                        while refs.len() > 0 {
                            let second_node_id: i64 = refs.next().unwrap();

                            edges.push(EdgeInfo {
                                first_node_id: first_node_id,
                                second_node_id: second_node_id,
                                tags: tags.clone(),
                            });
                            first_node_id = second_node_id;
                        }
                    }
                }
                _ => (),
            }
        })
        .unwrap();

    println!("Filtering out unecessary nodes...");

    let connected_nodes: HashSet<_> = edges
        .iter()
        .flat_map(|edge| vec![edge.first_node_id, edge.second_node_id])
        .collect();

    // filter out nodes not connected by an edge
    nodes.retain(|node| connected_nodes.contains(&node.id));

    println!("Inserting nodes into database...");

    let query = "CREATE (node:n {id: $id, lat: $lat, lng: $lng})";

    let progress = ProgressBar::new(nodes.len() as u64);

    // connection.set_lazy(true);
    // connection.set_autocommit(false);

    for node in nodes {
        let mut params = HashMap::new();
        params.insert("id".to_string(), QueryParam::Int(node.id));
        params.insert("lat".to_string(), QueryParam::Float(node.lat));
        params.insert("lng".to_string(), QueryParam::Float(node.lng));
        // param_map.insert("tags".to_string(), QueryParam::from(node.tags.clone()));

        progress.inc(1);

        connection.execute(query, Some(&params)).unwrap();
        connection.fetchall();
    }

    progress.finish(); // Finish the progress bar

    connection.commit();
    // connection::finalize()
}
