use cel_interpreter::{Context, Program, Value};
use std::collections::HashMap;
use serde::{Serialize, Serializer};

use crate::elements::{Element, Member, ElementType};

#[derive(Serialize)]
#[serde(remote = "Member")]
pub struct MemberDef {
    #[serde(rename = "type")]
    pub t: Option<String>,
    #[serde(rename = "ref")]
    pub id: i64,
    pub role: Option<String>,
}

#[derive(Serialize)]
#[serde(remote = "ElementType", tag = "type", rename_all = "lowercase")]
pub enum ElementTypeDef {
    Node {
        lat: f64,
        lon: f64,
    },
    Way {
        nodes: Vec<i64>,
    },
    Relation {
        #[serde(serialize_with = "serialize_member_vec")]
        members: Vec<Member>,
    },
}

fn serialize_member_vec<S: Serializer>(v: &[Member], serializer: S) -> Result<S::Ok, S::Error> {
    #[derive(Serialize)]
    struct Wrapper<'a>(#[serde(with = "MemberDef")] &'a Member);

    v.iter()
        .map(Wrapper)
        .collect::<Vec<_>>()
        .serialize(serializer)
}


fn _skip_visibility(visibility: &Option<bool>) -> bool {
    visibility.unwrap_or(true)
}

#[derive(Serialize)]
#[serde(remote = "Element")]
pub struct ElementDef {
    pub changeset: Option<i64>,
    pub user: Option<String>,
    pub version: Option<i32>,
    pub uid: Option<i32>,
    pub id: i64,
    pub timestamp: Option<String>,
    #[serde(skip_serializing_if = "_skip_visibility")]
    pub visible: Option<bool>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub tags: HashMap<String, String>,
    #[serde(flatten, with = "ElementTypeDef")]
    pub element_type: ElementType,
}

#[derive(Serialize)]
pub struct ElementWrapper(#[serde(with = "ElementDef")] Element);

pub struct CelFilter(Program);

fn convert_filter_output(value: &Value) -> Option<Element> {
    println!("{value:?}");
    None
}

impl CelFilter {
    pub fn evaluate(&self, element: Element) -> Option<Element> {
        let mut context = Context::default();
        context.add_variable("element", ElementWrapper(element)).unwrap();
        match &self.0.execute(&context) {
            Ok(o) => convert_filter_output(o),
            Err(e) => {
                eprintln!("Unable to execute filter for element: {e:?}, skipping...");
                None
            }
        }
    }
}

pub fn compile_cel_filter(filter_content: &str) -> Option<CelFilter> {
    let program = match Program::compile(filter_content) {
        Ok(p) => p,
        Err(e) => {
            panic!("Error parsing CEL filter: {e:?}");
        }
    };
    Some(CelFilter(program))
}
