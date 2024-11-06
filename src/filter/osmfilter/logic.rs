use ustr::Ustr;

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
        key: Ustr,
    },
    Equals {
        key: Ustr,
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
        SelectorStatement::Has { key } => element.tags.contains_key(key),
        SelectorStatement::Equals { key, value } => match element.tags.get(key) {
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
        keys: Vec<Ustr>,
    },
    KeepStatement {
        keys: Vec<Ustr>,
    },
    SetStatement {
        key: Ustr,
        value: String,
    },
    RenameStatement {
        old_key: Ustr,
        new_key: Ustr,
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
            element.tags.insert(*key, value.to_owned());
            StatementResult::Continue
        }
        Statement::RenameStatement { old_key, new_key } => {
            if let Some(v) = element.tags.remove(old_key) {
                element.tags.insert(*new_key, v);
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
