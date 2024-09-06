use std::collections::HashMap;
use std::io::BufRead;
use std::sync::mpsc::Sender;

use crate::elements::{Element, ElementType, Member, Metadata, SimpleElementType};

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

fn unescape_str(input: &str) -> String {
    let mut output = String::new();
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            let mut hex = String::new();
            while let Some(&next_char) = chars.peek() {
                if next_char == '%' {
                    chars.next(); // consume the closing '%'
                    break;
                }
                hex.push(chars.next().unwrap());
            }
            if let Ok(code_point) = u32::from_str_radix(&hex, 16) {
                if let Some(out_char) = std::char::from_u32(code_point) {
                    output.push(out_char);
                }
            }
        } else {
            output.push(c);
        }
    }
    output
}

fn add_field(field: &str, opl_element: &mut OplElement) {
    let (flag, value) = field.split_at(1);
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
            opl_element.username = Some(unescape_str(value));
        }
        "T" => {
            let tags: HashMap<String, String> = value
                .split(',')
                .filter_map(|t| t.split_once('='))
                .map(|(k, v)| (unescape_str(k), unescape_str(v)))
                .collect();
            opl_element.tags = Some(tags);
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
            let nodes: Vec<i64> = value
                .split(',')
                .map(|node_entry| {
                    let parts: Vec<&str> = node_entry.split(|c| c == 'x' || c == 'y').collect();
                    parts[0][1..].parse::<i64>().unwrap()
                })
                .collect();

            opl_element.element_type = Some(OplElementType::Way { nodes: Some(nodes) });
        }
        "M" => {
            let members: Vec<Member> = value
                .split(',')
                .filter_map(|member| {
                    let (ref_part, role) = member.split_once('@').unwrap();
                    let (type_char, member_id) = ref_part.split_at(1);
                    let member_type = match type_char {
                        "n" => SimpleElementType::Node,
                        "w" => SimpleElementType::Way,
                        "r" => SimpleElementType::Relation,
                        _ => return None,
                    };
                    Some(Member {
                        t: Some(member_type),
                        id: member_id.parse().ok().unwrap(),
                        role: Some(unescape_str(role)),
                    })
                })
                .collect();
            opl_element.element_type = Some(OplElementType::Relation {
                members: Some(members),
            });
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

pub fn read_opl<S: BufRead>(sender: Sender<Element>, metadata_sender: Sender<Metadata>, src: S) {
    eprintln!("Reading OPL input...");

    // create an empty Metadata object
    let metadata = Metadata::default();

    // send metadata to main thread
    metadata_sender
        .send(metadata)
        .expect("Couldn't send metdata to main thread!");

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unescape_str() {
        assert_eq!(unescape_str("%20%"), String::from(" "));
        assert_eq!(unescape_str("%2c%"), String::from(","));
        assert_eq!(unescape_str("%2c%%2c%"), String::from(",,"));
        assert_eq!(unescape_str("%1f631%"), String::from("😱"));
        assert_eq!(unescape_str("%12108%"), String::from("𒄈"));
    }
}
