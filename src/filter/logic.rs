use crate::elements;

#[derive(Debug)]
pub enum Element {
    Modifiable(elements::Element),
    Committed(elements::Element),
    None,
}

pub struct Equals {
    pub key: String,
    pub value: String,
}

pub enum Modifier {
    Set,
    Rename,
    Delete,
}

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

fn test_selector(selector: &SelectorStatement, element: &elements::Element) -> bool {
    match selector {
        SelectorStatement::Type { node, way, relation } => {
            match &element.element_type {
                elements::ElementType::Node { latitude, longitude } => {
                    node.to_owned()
                },
                elements::ElementType::Way { nodes } => {
                    way.to_owned()
                },
                elements::ElementType::Relation { references } => {
                    relation.to_owned()
                },
            }
        },
        SelectorStatement::Has { key } => {
            element.tags.contains_key(key.as_str())
        },
        SelectorStatement::Equals { key, value } => {
            match element.tags.get(key.as_str()) {
                Some(v) => {
                    v == value
                },
                _ => false,
            }
        },
    }
}

#[derive(Debug)]
pub enum Statement {
    CommitStatement,
    DropStatement,
    DeleteStatement {
        key: String,
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

pub fn evaluate_statement(statement: &Statement, element: Element) -> Element {
    match element {
        Element::Modifiable(mut e) => {
            match statement {
                Statement::CommitStatement => {
                    Element::Committed(e)
                },
                Statement::DropStatement => {
                    Element::None
                },
                Statement::DeleteStatement { key } => {
                    e.tags.remove(key.as_str());
                    Element::Modifiable(e)
                },
                Statement::SetStatement { key, value } => {
                    e.tags.insert(key.to_owned(), value.to_owned());
                    Element::Modifiable(e)
                },
                Statement::RenameStatement { old_key, new_key } => {
                    match e.tags.remove(old_key.as_str()) {
                        Some(v) => {
                            e.tags.insert(new_key.to_owned(), v);
                            ()
                        }
                        _ => ()
                    }
                    Element::Modifiable(e)
                },
                Statement::SelectionBlock { selector, statements } => {
                    if test_selector(selector, &e) {
                        let mut current_element = Element::Modifiable(e);
                        for current_statement in statements {
                            current_element = evaluate_statement(current_statement, current_element);
                            match current_element {
                                Element::Modifiable(_) => {},
                                _ => {
                                    return current_element;
                                },
                            }
                        }
                        current_element
                    } else {
                        Element::Modifiable(e)
                    }
                },
            }
        },
        e => e,
    }
}

#[derive(Debug)]
pub struct Filter {
    pub statements: Vec<Statement>
}
