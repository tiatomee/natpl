use natpl::{
    parsing::Parser,
    runtime::{CallStack, EvalResult, ItemError, Runtime},
    syntax::{Expression, Name},
    tokenising,
    value_unit::Unit,
    value_unit::{Value, ValueKind},
};

mod unit_search;

use std::collections::BTreeMap;

use crate::unit_search::unit_matches;

#[derive(Debug)]
pub enum ReadEvalResult {
    ParseError(String),
    ItemError(ItemError),

    Empty,
    SilentValue {
        value: Value,
        display_candidates: Vec<(String, ValueKind)>,
    },
    PrintValue {
        expr: Expression,
        value: Value,
        display_candidates: Vec<(String, ValueKind)>,
    },
    VariableSearchResult {
        unit_aliases: BTreeMap<Name, Value>,
        variables: BTreeMap<Name, Vec<(String, ValueKind)>>,
        functions: BTreeMap<Name, Vec<Name>>,
    },
    UnitSearchResult {
        unit: Unit,
        unit_aliases: BTreeMap<Name, ValueKind>,
        variables: BTreeMap<Name, Vec<(String, ValueKind)>>,
    },
}

pub fn read_eval(rt: &mut Runtime, input: &str) -> ReadEvalResult {
    let toks = tokenising::tokenise(&input);
    match Parser::parse_line(&toks) {
        Ok(line) => {
            let res = match rt.eval_line_item(line, &mut CallStack::default()) {
                Ok(res) => res,
                Err(err) => return ReadEvalResult::ItemError(err),
            };

            match res {
                EvalResult::Empty => ReadEvalResult::Empty,
                EvalResult::Value(val) => ReadEvalResult::SilentValue {
                    display_candidates: unit_matches(rt, &val).collect(),
                    value: val,
                },
                EvalResult::PrintValue(expr, val) => ReadEvalResult::PrintValue {
                    expr,
                    display_candidates: unit_matches(rt, &val).collect(),
                    value: val,
                },
                EvalResult::VariableSearchResult {
                    unit_aliases,
                    variables,
                    functions,
                } => ReadEvalResult::VariableSearchResult {
                    unit_aliases,
                    variables: variables
                        .into_iter()
                        .map(|(k, val)| (k, unit_matches(rt, &val).collect()))
                        .collect(),
                    functions,
                },
                EvalResult::UnitSearchResult {
                    unit,
                    unit_aliases,
                    variables,
                } => ReadEvalResult::UnitSearchResult {
                    unit_aliases,
                    variables: variables
                        .into_iter()
                        .map(|(k, val_kind)| {
                            let val = Value {
                                unit: unit.clone(),
                                kind: val_kind,
                            };
                            (k, unit_matches(rt, &val).collect())
                        })
                        .collect(),
                    unit,
                },
            }
        }
        Err(err) => ReadEvalResult::ParseError(err.to_string()),
    }
}
