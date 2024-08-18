use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;

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
            // FIXME!!!
            println!("encountered selection block.");
            println!("{pair:?}");
            logic::Statement::CommitStatement
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

// fn parse(filter_content: &str) -> Box<dyn Fn(Box<dyn elements::Element + Send + Sync>) -> Box<dyn elements::Element + Send + Sync>> {
pub fn parse_filter(filter_content: &str) {
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
                    let filter = _interpret_body(a);
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

    // filter.evaluate
}
