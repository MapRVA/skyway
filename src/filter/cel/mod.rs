use crate::filter::ElementFilter;
use cel_interpreter::{Context, Program, Value};

use crate::elements::{Element, ElementType};

pub struct CelFilter(Program);

fn convert_filter_output(value: &Value, element: Element) -> Option<Element> {
    match value {
        Value::Bool(keep_element) => {
            if *keep_element {
                Some(element)
            } else {
                None
            }
        }
        _ => {
            panic!("Unexpected output from CEL filter (not a boolean): {value:?}");
        }
    }
}

fn _generate_context<'a>(element: &Element) -> Context<'a> {
    let mut context = Context::default();
    context
        .add_variable("tags", element.tags.to_owned())
        .unwrap();
    context
        .add_variable("changeset", element.changeset)
        .unwrap();
    context
        .add_variable("user", element.user.to_owned())
        .unwrap();
    context.add_variable("uid", element.uid).unwrap();
    context.add_variable("id", element.id).unwrap();
    context
        .add_variable("timestamp", element.timestamp.to_owned())
        .unwrap();
    context.add_variable("visible", element.visible).unwrap();
    context
        .add_variable(
            "type",
            match element.element_type {
                ElementType::Node { .. } => "node",
                ElementType::Way { .. } => "way",
                ElementType::Relation { .. } => "relation",
            },
        )
        .unwrap();
    context
}

impl ElementFilter for CelFilter {
    fn evaluate(&self, element: Element) -> Option<Element> {
        let context = _generate_context(&element);
        match &self.0.execute(&context) {
            Ok(o) => convert_filter_output(o, element),
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
