

struct TagFilter {
    key: String,
    values: Vec<String>,
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

// fn create_filter_table(table: &toml::Table) -> FilterTable {
//     let mut filters = Vec::new();
//     for (key, values) in table.iter() {
//         let mut value_strings = Vec::new();
//         for value in values.as_array().unwrap() {
//             value_strings.push(value.as_str().unwrap().to_owned());
//         }
//         filters.push(TagFilter {
//             key: key.to_string(),
//             values: value_strings,
//         });
//     }
//     FilterTable { filters: filters }
// }
