use crate::elements::{Element, ElementType};
use crate::filter::ElementFilter;

#[derive(Debug)]
pub enum SelectorStatement {
    Type {
        node: bool,
        way: bool,
        relation: bool,
    },
    Has {
        key: String,
    },
    Equals {
        key: String,
        value: String,
    },
}

fn test_selector(selector: &SelectorStatement, element: &Element) -> bool {
    match selector {
        SelectorStatement::Type {
            node,
            way,
            relation,
        } => match &element.element_type {
            ElementType::Node { .. } => node.to_owned(),
            ElementType::Way { .. } => way.to_owned(),
            ElementType::Relation { .. } => relation.to_owned(),
        },
        SelectorStatement::Has { key } => element.tags.contains_key(key.as_str()),
        SelectorStatement::Equals { key, value } => match element.tags.get(key.as_str()) {
            Some(v) => v == value,
            _ => false,
        },
    }
}

#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
pub enum Statement {
    CommitStatement,
    DropStatement,
    DeleteStatement {
        keys: Vec<String>,
    },
    KeepStatement {
        keys: Vec<String>,
    },
    SetStatement {
        key: String,
        value: String,
    },
    RenameStatement {
        old_key: String,
        new_key: String,
    },
    SelectionBlock {
        selector: SelectorStatement,
        statements: Vec<Statement>,
    },
}

enum StatementResult {
    Continue,
    Commit,
    Drop,
}

fn evaluate_statement(statement: &Statement, element: &mut Element) -> StatementResult {
    match statement {
        Statement::CommitStatement => StatementResult::Commit,
        Statement::DropStatement => StatementResult::Drop,
        Statement::DeleteStatement { keys } => {
            for key in keys {
                element.tags.remove(key);
            }
            StatementResult::Continue
        }
        Statement::KeepStatement { keys } => {
            element.tags.retain(|k, _| keys.contains(k));
            StatementResult::Continue
        }
        Statement::SetStatement { key, value } => {
            element.tags.insert(key.to_owned(), value.to_owned());
            StatementResult::Continue
        }
        Statement::RenameStatement { old_key, new_key } => {
            if let Some(v) = element.tags.remove(old_key) {
                element.tags.insert(new_key.to_owned(), v);
            }
            StatementResult::Continue
        }
        Statement::SelectionBlock {
            selector,
            statements,
        } => {
            if test_selector(selector, element) {
                for sub_statement in statements {
                    match evaluate_statement(sub_statement, element) {
                        StatementResult::Continue => {}
                        result => return result,
                    }
                }
            }
            StatementResult::Continue
        }
    }
}

#[derive(Debug)]
pub struct OsmFilter {
    pub statements: Vec<Statement>,
}

impl ElementFilter for OsmFilter {
    fn evaluate(&self, mut element: Element) -> Option<Element> {
        for statement in &self.statements {
            match evaluate_statement(statement, &mut element) {
                StatementResult::Continue => {}
                StatementResult::Commit => return Some(element),
                StatementResult::Drop => return None,
            }
        }
        Some(element)
    }
}
