use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use json;

use skyway::elements;

fn translate_element(element: &json::JsonValue) -> elements::Element {
    let element_type: &str = element.remove("type").as_str()
        .expect("Unable to convert find element type string");

    let version = elements::Version {
        version: element.remove("version").as_u32().unwrap(),
        user: Null,
        timestamp: Null,
        uid: Null,
    };

    let mut tags: Vec<elements::Tag> = Vec::new();

    // TODO: iterate over element's tags, adding them to vector

    match element_type {
        "node" => elements::Node {
            id: element.remove("id").as_u64().unwrap(),
            version: version,
            tags: tags,
            latitude: element.remove("latitude").as_f64().unwrap(),
            longitude: element.remove("longitude").as_f64().unwrap(),
        }
        "way" => elements::Way {
            id: element.remove("id").as_u64().unwrap(),
            version: version,
            tags: tags,
            // nodes: ...
        }
        "relation" => elements::Relation {
            id: element.remove("id").as_u64().unwrap(),
            version: version,
            tags: tags,
            // references: 
        }
        _ => Err("Element type not recognized!")
    }
    element;
}

pub fn load_json(file_path: &Path) -> impl Iterator<Item = elements::Element> {
    let display = file_path.display();

    // attempt to open JSON input file
    let mut file = match File::open(file_path) {
        Err(why) => panic!("Couldn't open {}: {}", display, why),
        Ok(file) => file,
    };

    // read JSON input file into string s
    let mut s = String::new();
    file.read_to_string(&mut s)
        .expect("Error converting file to string.");

    // attempt to parse string as JSON
    let mut json_repr = json::parse(&s)
        .expect("Error parsing JSON string");

    // is the top-level JsonValue an object?
    if json_repr.is_object() {

        // determine the version of this OSM JSON document
        let version_value = json_repr.remove("version");
        if version_value.is_number() {
            let version = version_value.as_f32()
                .expect("Error converting version value to float32");
            // TODO: some kind of version check for compatibility?
            println!("Version of this OSM JSON is {}", version.to_string());
        }

        // transform iterator of JSON objects into iterator of Elements
        let document_elements = json_repr.remove("elements");
        return document_elements.members().map(|m| translate_element(m));

    } else {
        panic!("Parsed JSON was not an object!")
    }
}
