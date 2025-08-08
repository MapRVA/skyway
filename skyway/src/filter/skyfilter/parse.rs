use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;
use regex::Regex;

use crate::{
    SkywayError,
    filter::skyfilter::logic::{SelectorStatement, SkyFilter, Statement, StringOrRegex},
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[grammar = "filter/skyfilter/skyfilter.pest"]
struct SkyFilterParser;

fn get_inner_string(pair: Pair<Rule>) -> String {
    pair.into_inner()
        .next()
        .unwrap()
        .as_span()
        .as_str()
        .to_owned()
}

fn get_inner_string_or_regex(pair: Pair<Rule>) -> StringOrRegex {
    match pair.as_rule() {
        Rule::quoted_string => StringOrRegex::String(get_inner_string(pair)),
        Rule::regex => StringOrRegex::Regex(Regex::new(&get_inner_string(pair)).unwrap()), // TODO: better error handling here
        _ => unreachable!(),
    }
}

fn collect_inner_strings_or_regexes(pair: Pair<Rule>) -> Vec<StringOrRegex> {
    pair.into_inner()
        .map(|x| get_inner_string_or_regex(x))
        .collect()
}

fn parse_set_statement(pair: Pair<Rule>) -> Statement {
    let mut inner = pair.into_inner();
    Statement::SetStatement {
        key: get_inner_string(inner.next().unwrap()),
        value: get_inner_string(inner.next().unwrap()),
    }
}

fn parse_rename_statement(pair: Pair<Rule>) -> Statement {
    let mut inner = pair.into_inner();
    Statement::RenameStatement {
        old_key: get_inner_string(inner.next().unwrap()),
        new_key: get_inner_string(inner.next().unwrap()),
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
        key: get_inner_string(inner.next().unwrap()),
        value: get_inner_string_or_regex(inner.next().unwrap()),
    }
}

fn parse_selector(pair: Pair<Rule>) -> SelectorStatement {
    match pair.as_rule() {
        Rule::has => SelectorStatement::Has {
            key: get_inner_string_or_regex(pair.into_inner().next().unwrap()),
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
            keys: collect_inner_strings_or_regexes(pair),
        },
        Rule::set => parse_set_statement(pair),
        Rule::keep => Statement::KeepStatement {
            keys: collect_inner_strings_or_regexes(pair),
        },
        Rule::rename => parse_rename_statement(pair),
        Rule::selection_block => parse_selection_block(pair),
        _ => unreachable!(),
    }
}

fn _interpret_body(body: Pair<Rule>) -> SkyFilter {
    match body.as_rule() {
        Rule::body => {
            let mut statements = Vec::new();
            for pair in body.into_inner() {
                statements.push(interpret_statement(pair));
            }
            SkyFilter { statements }
        }
        _ => unreachable!(),
    }
}

pub fn parse_filter(filter_content: &str) -> Result<SkyFilter, SkywayError> {
    let mut file = match SkyFilterParser::parse(Rule::file, filter_content) {
        Ok(v) => v,
        Err(e) => {
            return Err(SkywayError::UnparsableFilter(e.to_string()));
        }
    };

    match file.next() {
        Some(a) => {
            let rule = a.as_rule();
            match rule {
                Rule::version => {
                    let filter_version = a.as_str();
                    if filter_version != VERSION {
                        eprintln!(
                            "WARNING: Version mismatch, the filter is version {} but you are running skyway {}. You may encounter unexpected behavior.",
                            filter_version, VERSION
                        );
                    }
                }
                _ => unreachable!(),
            }
        }
        _ => unreachable!(),
    }

    match file.next() {
        Some(a) => match a.as_rule() {
            Rule::body => Ok(_interpret_body(a)),
            _ => unreachable!(),
        },
        _ => unreachable!(),
    }
}
