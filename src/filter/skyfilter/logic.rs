use regex::Regex;

use crate::elements::{Element, ElementType};
use crate::filter::ElementFilter;

#[derive(Debug)]
pub enum StringOrRegex {
    String(String),
    Regex(Regex),
}

impl PartialEq<String> for StringOrRegex {
    fn eq(&self, other: &String) -> bool {
        match self {
            StringOrRegex::String(s) => s.eq(other),
            StringOrRegex::Regex(r) => r.is_match(other),
        }
    }
}

#[derive(Debug)]
pub enum SelectorStatement {
    Type {
        node: bool,
        way: bool,
        relation: bool,
    },
    Has {
        key: StringOrRegex,
    },
    Equals {
        key: String,
        value: StringOrRegex,
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
        SelectorStatement::Has { key } => match key {
            StringOrRegex::String(s) => element.tags.contains_key(s),
            StringOrRegex::Regex(r) => element.tags.keys().any(|k| r.is_match(k)),
        },
        SelectorStatement::Equals { key, value } => match element.tags.get(key) {
            Some(v) => value == v,
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
        keys: Vec<StringOrRegex>,
    },
    KeepStatement {
        keys: Vec<StringOrRegex>,
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
            element.tags.retain(|k, _| !keys.iter().any(|e| e == k));
            StatementResult::Continue
        }
        Statement::KeepStatement { keys } => {
            element.tags.retain(|k, _| keys.iter().any(|e| e == k));
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
pub struct SkyFilter {
    pub statements: Vec<Statement>,
}

impl ElementFilter for SkyFilter {
    fn evaluate(&self, element: &mut Element) -> bool {
        for statement in &self.statements {
            match evaluate_statement(statement, element) {
                StatementResult::Continue => {}
                StatementResult::Commit => return true,
                StatementResult::Drop => return false,
            }
        }
        true // commit element if we've exhausted all statements
    }
}
