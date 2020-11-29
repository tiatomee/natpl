use std::collections::HashMap;

use crate::{
    parsing::Parser,
    syntax::{HasFC, Name, SiPrefix},
    value_unit::{Value, ValueKind},
};
use bigdecimal::{BigDecimal, ToPrimitive};
use owo_colors::OwoColorize;
use rustyline::error::ReadlineError;

use crate::{
    runtime::{EvalResult, Runtime},
    tokenising,
};

const EXPR_PAD: usize = 12;

pub fn repl(rt: &mut Runtime) -> Result<(), Box<dyn std::error::Error>> {
    let mut rl = rustyline::Editor::<()>::new();

    let prompt = format!("{} ", ">".blue());

    loop {
        let readline = rl.readline(&prompt);
        match readline {
            Ok(line) => {
                if line.is_empty() {
                    continue;
                }

                rl.add_history_entry(&line);

                let toks = tokenising::tokenise(&line);
                let line_item = Parser::parse_line(&toks);

                match line_item {
                    Ok(item) => match rt.eval_line_item(item) {
                        Ok(EvalResult::Empty) => {}
                        Ok(EvalResult::VariableSearchResult {
                            unit_aliases: _,
                            variables,
                            functions: _,
                        }) => {
                            let longest_name =
                                variables.keys().max_by(|a, b| a.len().cmp(&b.len()));
                            let name_length = longest_name.map(|s| s.len()).unwrap_or(0);

                            for (i, (name, val)) in variables.into_iter().enumerate() {
                                let marker = if i == 0 {
                                    format!("{}", "✔".green())
                                } else {
                                    format!("{}", "⇒".bright_black())
                                };

                                let (unit_name, val_kind) = unit_matches(rt, &val).next().unwrap();
                                let line = format!(
                                    "{:width$} = {:expr_pad$} [{}]",
                                    name,
                                    val_kind.to_string(),
                                    unit_name,
                                    width = name_length,
                                    expr_pad = EXPR_PAD
                                );
                                println!("{} {}", marker, line.bright_black());
                            }
                        }
                        Ok(EvalResult::UnitSearchResult {
                            unit,
                            unit_aliases: _,
                            variables,
                        }) => {
                            let longest_name =
                                variables.keys().max_by(|a, b| a.len().cmp(&b.len()));
                            let name_length = longest_name.map(|s| s.len()).unwrap_or(0);

                            for (i, (name, kind)) in variables.into_iter().enumerate() {
                                let marker = if i == 0 {
                                    format!("{}", "✔".green())
                                } else {
                                    format!("{}", "⇒".bright_black())
                                };

                                let val = Value {
                                    kind,
                                    unit: unit.clone(),
                                };
                                let (unit_name, val_kind) = unit_matches(rt, &val).next().unwrap();
                                let line = format!(
                                    "{:width$} = {:expr_pad$} [{}]",
                                    name,
                                    val_kind.to_string(),
                                    unit_name,
                                    width = name_length,
                                    expr_pad = EXPR_PAD
                                );
                                println!("{} {}", marker, line.bright_black());
                            }
                        }
                        Ok(EvalResult::Value(Value {
                            kind: ValueKind::Bool(true),
                            ..
                        })) => {
                            eprintln!("{} {}", "✔".green(), "true".bright_black());
                        }
                        Ok(EvalResult::Value(Value {
                            kind: ValueKind::Bool(false),
                            ..
                        })) => {
                            eprintln!("{} {}", "✘".red(), "false".bright_black());
                        }
                        Ok(EvalResult::Value(val)) => {
                            let candidates = unit_matches(rt, &val);

                            for (i, (unit, val)) in candidates.take(3).enumerate() {
                                let marker = if i == 0 {
                                    format!("{}", "✔".green())
                                } else {
                                    format!("{}", "⇒".bright_black())
                                };

                                let response = format!(
                                    "{:expr_pad$} [{}]",
                                    val.to_string(),
                                    unit,
                                    expr_pad = 12
                                );
                                println!("{} {}", marker, response.bright_black());
                            }
                        }
                        Ok(EvalResult::PrintValue(expr, val)) => {
                            let range = expr.fc();
                            let input_slice = &line[range.start..range.end];

                            let value_part = format!("{} => {}", input_slice, val.kind);
                            let response = format!(
                                "{:expr_pad$} [{}]",
                                value_part,
                                val.unit,
                                expr_pad = EXPR_PAD
                            );
                            println!("{} {}", "✔".green(), response.bright_black());
                        }
                        Err(err) => {
                            eprintln!("{} {}", "✘".red(), err);
                        }
                    },
                    Err(err) => {
                        eprintln!("Parse error: {}", err);
                        continue;
                    }
                }
            }
            Err(ReadlineError::Interrupted) => break,
            Err(ReadlineError::Eof) => break,
            Err(err) => {
                eprintln!("Error: {}", err);
                break;
            }
        }
    }

    Ok(())
}

type Rating = u64;

fn unit_matches(rt: &Runtime, val: &Value) -> impl Iterator<Item = (Name, ValueKind)> {
    fn name_with_prefix(name: &str, prefix: Option<SiPrefix>) -> String {
        if let Some(pre) = prefix {
            if name.len() <= 2 {
                format!("{}{}", pre.short_prefix(), name)
            } else {
                format!("{}{}", pre.full_prefix(), name)
            }
        } else {
            name.to_string()
        }
    }

    let val = val.clone();

    let units = rt.find_units(&val.unit);

    let mut matches = closest_match(&val.kind, units);

    // Iterator speghetti!!
    // Try to take the best candidate, if it's actually anything
    // unique (not `s` vs second)
    matches
        .next()
        .into_iter()
        .filter({
            let val = val.clone();
            move |(_, _, val_kind)| val_kind != &val.kind
        })
        .map(|(name, pre, val_kind)| (name_with_prefix(&name, pre), val_kind))
        // Then take the "raw" unit, this one is always printed
        // as a kind of fallback in case the suggestion is not good.
        .chain(Some((val.unit.to_string(), val.kind.clone())).into_iter())
        // Then the rest of the candidates (that are unique)
        .chain(
            matches
                .filter({
                    let val = val.clone();
                    move |(_, _, val_kind)| val_kind != &val.kind
                })
                .map(|(name, pre, val_kind)| (name_with_prefix(&name, pre), val_kind)),
        )
}

fn closest_match(
    val_kind: &ValueKind,
    matches: HashMap<&Name, ValueKind>,
) -> impl Iterator<Item = (Name, Option<SiPrefix>, ValueKind)> {
    let prefixes = &[
        SiPrefix::Femto,
        SiPrefix::Pico,
        SiPrefix::Nano,
        SiPrefix::Micro,
        SiPrefix::Milli,
        SiPrefix::Centi,
        // These three often end up doing weird things
        // and are not used very often anyway 👀
        // SiPrefix::Deci,
        // SiPrefix::Deca,
        // SiPrefix::Hecto,
        SiPrefix::Kilo,
        SiPrefix::Mega,
        SiPrefix::Giga,
        SiPrefix::Tera,
        SiPrefix::Peta,
    ];

    let mut rating: HashMap<String, (Option<SiPrefix>, ValueKind, Rating)> = Default::default();

    'matches: for (name, val) in matches {
        let (v, m) = match (val_kind, &val) {
            (ValueKind::Number(a), ValueKind::Number(b)) => (a, b),
            _ => continue 'matches,
        };

        for prefix in prefixes {
            let m = m * prefix.value();

            let dist = num_distance(v, &m, 1.0);

            rating
                .entry(name.clone())
                .and_modify(|(pre, val, rating)| {
                    if dist < *rating {
                        *pre = Some(*prefix);
                        *val = ValueKind::Number(v / m);
                        *rating = dist;
                    }
                })
                .or_insert_with(|| (Some(*prefix), val.clone(), dist));
        }

        let dist = num_distance(v, &m, 0.5);

        rating
            .entry(name.clone())
            .and_modify(|(pre, val, rating)| {
                if dist < *rating {
                    *pre = None;
                    *val = ValueKind::Number(v / m);
                    *rating = dist;
                }
            })
            .or_insert_with(|| (None, val.clone(), dist));
    }

    let mut ratings = rating.into_iter().collect::<Vec<_>>();
    ratings.sort_by(|(na, (pa, _, ra)), (nb, (pb, _, rb))| {
        pa.map(|s| s.sort_towards_middle())
            .cmp(&pb.map(|s| s.sort_towards_middle()))
            .then(ra.cmp(rb))
            .then(na.len().cmp(&nb.len()))
    });
    ratings
        .into_iter()
        .map(|(name, (prefix, val, _))| (name, prefix, val))
}

fn num_distance(a: &BigDecimal, b: &BigDecimal, handicap: f64) -> Rating {
    let div = a / b;
    let div_abs = div.abs();

    if div_abs < BigDecimal::from(1) || div_abs > BigDecimal::from(999) {
        Rating::max_value()
    } else {
        (div_abs.to_f64().unwrap() * handicap) as Rating
    }
}
