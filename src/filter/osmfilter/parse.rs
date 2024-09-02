use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

use crate::filter::osmfilter::logic::{OsmFilter, SelectorStatement, Statement};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[grammar = "filter/osmfilter/osmfilter.pest"]
struct OSMFilterParser;

fn get_inner_string(pair: &Pair<Rule>) -> String {
    pair.as_span().as_str().to_owned()
}

fn collect_inner_strings(pair: Pair<Rule>) -> Vec<String> {
    pair.into_inner().map(|x| get_inner_string(&x)).collect()
}

fn parse_set_statement(pair: Pair<Rule>) -> Statement {
    let inner: Vec<Pair<Rule>> = pair.into_inner().collect();
    match &inner[..] {
        [key, value] => Statement::SetStatement {
            key: get_inner_string(key),
            value: get_inner_string(value),
        },
        _ => panic!("Invalid set statement"),
    }
}

fn parse_rename_statement(pair: Pair<Rule>) -> Statement {
    let inner: Vec<Pair<Rule>> = pair.into_inner().collect();
    match &inner[..] {
        [key, value] => Statement::RenameStatement {
            old_key: get_inner_string(key),
            new_key: get_inner_string(value),
        },
        _ => panic!("Invalid rename statement"),
    }
}

fn parse_type_selector(pair: Pair<Rule>) -> SelectorStatement {
    let types: Vec<Rule> = pair.into_inner().map(|p| p.as_rule()).collect();
    SelectorStatement::Type {
        node: types.contains(&Rule::node),
        way: types.contains(&Rule::way),
        relation: types.contains(&Rule::relation),
    }
}

fn parse_equals_selector(pair: Pair<Rule>) -> SelectorStatement {
    let mut inner = pair.into_inner();
    SelectorStatement::Equals {
        key: get_inner_string(&inner.next().unwrap()),
        value: get_inner_string(&inner.next().unwrap()),
    }
}

fn parse_selector(pair: Pair<Rule>) -> SelectorStatement {
    match pair.as_rule() {
        Rule::has => {
            SelectorStatement::Has {
                key: get_inner_string(&pair.into_inner().next().unwrap()),
            }
        },
        Rule::equals => parse_equals_selector(pair),
        Rule::type_selector => parse_type_selector(pair),
        _ => unreachable!(),
    }
}

fn parse_selection_block(pair: Pair<Rule>) -> Statement {
    let mut inner = pair.into_inner();
    let selector = parse_selector(inner.next().unwrap());
    let statements = inner.map(interpret_statement).collect();
    Statement::SelectionBlock {
        selector,
        statements,
    }
}

fn interpret_statement(pair: Pair<Rule>) -> Statement {
    match pair.as_rule() {
        Rule::commit => Statement::CommitStatement,
        Rule::drop => Statement::DropStatement,
        Rule::delete => Statement::DeleteStatement {
            keys: collect_inner_strings(pair),
        },
        Rule::set => parse_set_statement(pair),
        Rule::keep => Statement::KeepStatement {
            keys: collect_inner_strings(pair),
        },
        Rule::rename => parse_rename_statement(pair),
        Rule::selection_block => parse_selection_block(pair),
        _ => unreachable!(),
    }
}

fn _interpret_body(body: Pair<Rule>) -> OsmFilter {
    match body.as_rule() {
        Rule::body => {
            let mut statements = Vec::new();
            for pair in body.into_inner() {
                statements.push(interpret_statement(pair));
            }
            OsmFilter { statements }
        }
        _ => unreachable!(),
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
                }
                _ => unreachable!(),
            }
        }
        _ => unreachable!(),
    }

    match file.next() {
        Some(a) => match a.as_rule() {
            Rule::body => Some(_interpret_body(a)),
            _ => unreachable!(),
        },
        _ => unreachable!(),
    }
}
