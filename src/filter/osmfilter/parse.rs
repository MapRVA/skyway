use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;

use crate::filter::osmfilter::logic::{OsmFilter, SelectorStatement, Statement};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[grammar = "filter/osmfilter/osmfilter.pest"]
struct OSMFilterParser;

fn _interpret_statement(pair: Pair<Rule>) -> Statement {
    match pair.as_rule() {
        Rule::commit => {
            Statement::CommitStatement
        },
        Rule::drop => {
            Statement::DropStatement
        }, 
        Rule::delete => {
            let mut keys = Vec::new();
            for v in pair.into_inner() {
                keys.push(v
                    .as_span()
                    .as_str()
                    .to_owned())
            }
            Statement::DeleteStatement {
                keys,
            }
        },
        Rule::set => {
            let mut inner = pair.into_inner();
            Statement::SetStatement {
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
        Rule::keep => {
            let mut keys = Vec::new();
            for v in pair.into_inner() {
                keys.push(v
                    .as_span()
                    .as_str()
                    .to_owned())
            }
            Statement::KeepStatement {
                keys,
            }
        },
        Rule::rename => {
            let mut inner = pair.into_inner();
            Statement::RenameStatement {
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
            Statement::SelectionBlock {
                selector: match this_selector_rule {
                    Rule::has => {
                        SelectorStatement::Has {
                            key: this_selector_inner.next()
                                .unwrap()
                                .as_span()
                                .as_str()
                                .to_owned()
                        }
                    },
                    Rule::equals => {
                        SelectorStatement::Equals {
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
                        SelectorStatement::Type {
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


fn _interpret_body(body: Pair<Rule>) -> OsmFilter {
    match body.as_rule() {
        Rule::body => {
            let mut statements = Vec::new();
            for pair in body.into_inner() {
                statements.push(_interpret_statement(pair));
            }
            OsmFilter {
                statements
            }
        },
        _ => {
            unreachable!();
        },
    }
}

pub fn parse_filter(filter_content: &str) -> Option<OsmFilter> {
    let mut file = match OSMFilterParser::parse(Rule::file, filter_content) {
        Ok(v) => v,
        Err(_) => {
            return None;
        }
    };

    match file.next() {
        Some(a) => {
            let rule = a.as_rule();
            match rule {
                Rule::version => {
                    let filter_version = a.as_str();
                    if filter_version != VERSION {
                        eprintln!("WARNING: Version mismatch, the filter is version {} but you are running skyway {}. You may encounter unexpected behavior.", filter_version, VERSION);
                    }
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
                    Some(_interpret_body(a))
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
