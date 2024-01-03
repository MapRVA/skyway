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

let pb = ProgressBar::new(nodes.len() as u64);

// connection.set_lazy(true);
// connection.set_autocommit(false);

for node in nodes {
let mut params = HashMap::new();
params.insert("id".to_string(), QueryParam::Int(node.id));
params.insert("lat".to_string(), QueryParam::Float(node.lat));
params.insert("lng".to_string(), QueryParam::Float(node.lng));
// param_map.insert("tags".to_string(), QueryParam::from(node.tags.clone()));

pb.inc(1);

connection.execute(query, Some(&params)).unwrap();
connection.fetchall();
}

pb.finish();

connection.commit();

println!("Inserting edges into the database:");

let query = "MATCH (n1:n),(n2:n) WHERE n1.id = $first_id AND n2.id = $second_id CREATE (n1)-[r:CONNECTED_TO]->(n2)";

let pb = ProgressBar::new(edges.len() as u64);

for edge in edges {
let mut params = HashMap::new();
params.insert("first_id".to_string(), QueryParam::Int(edge.first_node_id));
params.insert("second_id".to_string(), QueryParam::Int(edge.second_node_id));
// param_map.insert("tags".to_string(), QueryParam::from(edge.tags.clone()));

pb.inc(1);

connection.execute(query, Some(&params)).unwrap();
connection.fetchall();
}

pb.finish();

connection.commit();
// connection::finalize();
