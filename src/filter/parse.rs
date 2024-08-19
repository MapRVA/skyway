use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;
use std::sync::mpsc::{Sender, Receiver};
use crate::elements;

use crate::filter::logic;

#[derive(Parser)]
#[grammar = "filter/osmfilter.pest"]
struct OSMFilterParser;

fn _interpret_statement(pair: Pair<Rule>) -> logic::Statement {
    match pair.as_rule() {
        Rule::commit => {
            logic::Statement::CommitStatement
        },
        Rule::drop => {
            logic::Statement::DropStatement
        }, 
        Rule::delete => {
            let mut inner = pair.into_inner();
            logic::Statement::DeleteStatement {
                key: inner.next()
                    .unwrap()
                    .as_span()
                    .as_str()
                    .to_owned(),
            }
        },
        Rule::set => {
            let mut inner = pair.into_inner();
            logic::Statement::SetStatement {
                key: inner.next()
                    .unwrap()
                    .as_span()
                    .as_str()
                    .to_owned(),
                value: inner.next()
                    .unwrap()
                    .as_span()
                    .as_str()
                    .to_owned(),
            }
        },
        Rule::rename => {
            let mut inner = pair.into_inner();
            logic::Statement::RenameStatement {
                old_key: inner.next()
                    .unwrap()
                    .as_span()
                    .as_str()
                    .to_owned(),
                new_key: inner.next()
                    .unwrap()
                    .as_span()
                    .as_str()
                    .to_owned(),
            }
        },
        Rule::selection_block => {
            let mut inner = pair.into_inner();
            let this_selector = inner.next()
                .unwrap()
                .into_inner()
                .next()
                .unwrap();
            let mut statements = Vec::new();
            for p in inner {
                statements.push(_interpret_statement(p));
            }
            let this_selector_rule = this_selector.as_rule();
            let mut this_selector_inner = this_selector.into_inner();
            logic::Statement::SelectionBlock {
                selector: match this_selector_rule {
                    Rule::has => {
                        logic::SelectorStatement::Has {
                            key: this_selector_inner.next()
                                .unwrap()
                                .as_span()
                                .as_str()
                                .to_owned()
                        }
                    },
                    Rule::equals => {
                        logic::SelectorStatement::Equals {
                            key: this_selector_inner.next()
                                .unwrap()
                                .as_span()
                                .as_str()
                                .to_owned(),
                            value: this_selector_inner.next()
                                .unwrap()
                                .as_span()
                                .as_str()
                                .to_owned(),
                        }
                    },
                    Rule::type_selector => {
                        let mut this_node = false;
                        let mut this_way = false;
                        let mut this_relation = false;
                        for p in this_selector_inner {
                            match p.as_rule() {
                                Rule::node => {
                                    this_node = true;
                                },
                                Rule::way => {
                                    this_way = true;
                                },
                                Rule::relation => {
                                    this_relation = true;
                                },
                                _ => {
                                    unreachable!();
                                },
                            }
                        }
                        logic::SelectorStatement::Type {
                            node: this_node,
                            way: this_way,
                            relation: this_relation,
                        }
                    },
                    _ => {
                        unreachable!();
                    },
                },
                statements,
            }
        },
        _ => {
            unreachable!();
        },
    }
}


fn _interpret_body(body: Pair<Rule>) -> logic::Filter {
    match body.as_rule() {
        Rule::body => {
            let mut statements = Vec::new();
            for pair in body.into_inner() {
                statements.push(_interpret_statement(pair));
            }
            logic::Filter {
                statements
            }
        },
        _ => {
            unreachable!();
        },
    }
}

pub fn parse_filter(filter_content: &str) -> logic::Filter {
    let mut file = match OSMFilterParser::parse(Rule::file, filter_content) {
        Ok(v) => v,
        Err(e) => {
            panic!("Unable to parse filter: {e:?}");
        }
    };

    match file.next() {
        Some(a) => {
            let rule = a.as_rule();
            match rule {
                Rule::version => {
                    println!("found version! skipping that for now...");
                },
                _ => {
                    unreachable!();
                }
            }
        },
        _ => {
            unreachable!();
        }
    }

    match file.next() {
        Some(a) => {
            match a.as_rule() {
                Rule::body => {
                    _interpret_body(a)
                },
                _ => {
                    unreachable!();
                }
            }
        },
        _ => {
            unreachable!();
        },
    }
}

fn _element_to_option(element: logic::Element) -> Option<elements::Element> {
    match element {
        logic::Element::Modifiable(e) => Some(e),
        logic::Element::Committed(e) => Some(e),
        logic::Element::None => None,
    }
}

fn evaluate_filter<'a>(filter: &mut logic::Filter, element: elements::Element) -> Option<elements::Element> {
    let mut current_element = logic::Element::Modifiable(element);
    for statement in &filter.statements {
        current_element = logic::evaluate_statement(statement, current_element);
        match current_element {
            logic::Element::Modifiable(_) => {},
            _ => {
                return _element_to_option(current_element)
            }
        }
    }
    _element_to_option(current_element)
}

pub fn filter_elements<'a>(filter_contents: &str, receiver: Receiver<elements::Element>, sender: Sender<elements::Element>) {
    let mut filter = parse_filter(filter_contents);
    for e in receiver.iter() {
        match evaluate_filter(&mut filter, e) {
            Some(v) => {
                sender.send(v);
            },
            _ => {},
        }
    }
}
