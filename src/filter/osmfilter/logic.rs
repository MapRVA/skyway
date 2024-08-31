use crate::elements;

#[derive(Debug)]
pub enum Element {
    Modifiable(elements::Element),
    Committed(elements::Element),
    None,
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
                elements::ElementType::Node { .. } => {
                    node.to_owned()
                },
                elements::ElementType::Way { .. } => {
                    way.to_owned()
                },
                elements::ElementType::Relation { .. } => {
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
                Statement::DeleteStatement { keys } => {
                    for key in keys {
                        e.tags.remove(key.as_str());
                    }
                    Element::Modifiable(e)
                },
                Statement::KeepStatement { keys } => {
                    e.tags = e.tags.iter()
                        .filter(|(k, _)| keys.contains(k))
                        .map(|(k, v)| (k.to_owned(), v.to_owned()))
                        .collect();
                    Element::Modifiable(e)
                },
                Statement::SetStatement { key, value } => {
                    e.tags.insert(key.to_owned(), value.to_owned());
                    Element::Modifiable(e)
                },
                Statement::RenameStatement { old_key, new_key } => {
                    if let Some(v) = e.tags.remove(old_key.as_str()) {
                        e.tags.insert(new_key.to_owned(), v);
                        
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

fn _element_to_option(element: Element) -> Option<elements::Element> {
    match element {
        Element::Modifiable(e) => Some(e),
        Element::Committed(e) => Some(e),
        Element::None => None,
    }
}

#[derive(Debug)]
pub struct OsmFilter {
    pub statements: Vec<Statement>
}

impl OsmFilter {
    pub fn evaluate(&self, element: elements::Element) -> Option<elements::Element> {
        let mut current_element = Element::Modifiable(element);
        for statement in &self.statements {
            current_element = evaluate_statement(statement, current_element);
            match current_element {
                Element::Modifiable(_) => {},
                _ => {
                    return _element_to_option(current_element)
                }
            }
        }
        _element_to_option(current_element)
    }
}
