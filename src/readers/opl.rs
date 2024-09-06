use std::collections::HashMap;
use std::io::BufRead;
use std::sync::mpsc::Sender;

use crate::elements::{Element, ElementType, Member, Metadata};

#[derive(Debug)]
enum OplElementType {
    Node { lat: Option<f64>, lon: Option<f64> },
    Way { nodes: Option<Vec<i64>> },
    Relation { members: Option<Vec<Member>> },
}

impl From<OplElementType> for ElementType {
    fn from(value: OplElementType) -> Self {
        match value {
            OplElementType::Node { lat, lon } => ElementType::Node {
                lat: lat.unwrap(),
                lon: lon.unwrap(),
            },
            OplElementType::Way { nodes } => ElementType::Way {
                nodes: nodes.unwrap(),
            },
            OplElementType::Relation { members } => ElementType::Relation {
                members: members.unwrap(),
            },
        }
    }
}

#[derive(Debug)]
struct OplElement {
    id: Option<i64>,
    version: Option<i32>,
    visible: Option<bool>,
    changeset: Option<i64>,
    timestamp: Option<String>,
    user_id: Option<i32>,
    username: Option<String>,
    tags: Option<HashMap<String, String>>,
    element_type: Option<OplElementType>,
}

impl OplElement {
    fn new() -> Self {
        OplElement {
            id: None,
            version: None,
            visible: None,
            changeset: None,
            timestamp: None,
            user_id: None,
            username: None,
            tags: None,
            element_type: None,
        }
    }
}

impl From<OplElement> for Element {
    fn from(value: OplElement) -> Self {
        let id = value.id.unwrap();
        let tags = value.tags.unwrap();
        let element_type = ElementType::from(value.element_type.unwrap());
        Element {
            id,
            tags,
            element_type,
            changeset: value.changeset,
            visible: value.visible,
            timestamp: value.timestamp,
            uid: value.user_id,
            user: value.username,
            version: value.version,
        }
    }
}

fn car_cdr(s: &str) -> (&str, &str) {
    match s.chars().next() {
        Some(c) => s.split_at(c.len_utf8()),
        None => s.split_at(0),
    }
}

fn add_field(field: &str, opl_element: &mut OplElement) {
    let (flag, value) = car_cdr(field);
    println!("{:?}", opl_element);
    match flag {
        "n" => {
            opl_element.id = Some(value.parse::<i64>().unwrap());
        }
        "w" => {
            opl_element.id = Some(value.parse::<i64>().unwrap());
        }
        "r" => {
            opl_element.id = Some(value.parse::<i64>().unwrap());
        }
        "v" => {
            opl_element.version = Some(value.parse::<i32>().unwrap());
        }
        "d" => match value {
            "V" => opl_element.visible = Some(true),
            "D" => opl_element.visible = Some(false),
            _ => {
                panic!("Deleted field value not recognized: {field}");
            }
        },
        "c" => {
            opl_element.changeset = Some(value.parse::<i64>().unwrap());
        }
        "t" => {
            opl_element.timestamp = Some(value.to_string());
        }
        "i" => {
            opl_element.user_id = Some(value.parse::<i32>().unwrap());
        }
        "u" => {
            opl_element.username = Some(value.to_string());
        }
        "T" => {
            unimplemented!();
            // let tags = HashMap::new();
            // FIXME
            // opl_element.tags = tags;
        }
        "x" => match opl_element.element_type {
            Some(OplElementType::Node { lat, .. }) => {
                opl_element.element_type = Some(OplElementType::Node {
                    lat,
                    lon: Some(value.parse::<f64>().unwrap()),
                });
            }
            None => {
                opl_element.element_type = Some(OplElementType::Node {
                    lat: None,
                    lon: Some(value.parse::<f64>().unwrap()),
                });
            }
            _ => {
                panic!("Longitude set for a non-node element!");
            }
        },
        "y" => match opl_element.element_type {
            Some(OplElementType::Node { lon, .. }) => {
                opl_element.element_type = Some(OplElementType::Node {
                    lat: Some(value.parse::<f64>().unwrap()),
                    lon,
                });
            }
            None => {
                opl_element.element_type = Some(OplElementType::Node {
                    lat: Some(value.parse::<f64>().unwrap()),
                    lon: None,
                });
            }
            _ => {
                panic!("Latitude set for a non-node element!");
            }
        },
        "N" => {
            unimplemented!();
        }
        "M" => {
            unimplemented!();
        }
        _ => {
            panic!("Unrecognized field: {field}");
        }
    }
}

fn convert_element(line: String) -> Element {
    let mut opl_element = OplElement::new();
    line.split_whitespace()
        .for_each(|x| add_field(x, &mut opl_element));
    Element::from(opl_element)
}

pub fn read_opl<S: BufRead>(sender: Sender<Element>, _metadata_sender: Sender<Metadata>, src: S) {
    eprintln!("Reading OPL input...");

    for line in src.lines().take_while(|l| l.is_ok()) {
        let line = line.unwrap();
        match sender.send(convert_element(line)) {
            Ok(_) => (),
            Err(e) => {
                panic!("ERROR: Unable to send an element: {e:?}");
            }
        }
    }
}
