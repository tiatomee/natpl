use std::collections::HashMap;

use bigdecimal::{BigDecimal, ToPrimitive};
use runtime::{EvalResult, Runtime};
use rustyline::error::ReadlineError;

use owo_colors::OwoColorize;
use term::ValueKind;
use thiserror::Error;

use syntax::{HasFC, Name, SiPrefix};

pub mod syntax;

pub mod parsing;
pub mod tokenising;

pub mod runtime;
pub mod term;

pub mod util;

#[derive(Debug, Error)]
pub enum LoadFileError {
    #[error("Runtime error: {}", .0)]
    RuntimeError(#[from] runtime::ItemError),
    #[error("Parse error {}", .0)]
    ParseError(String),
}

fn load_file(rt: &mut Runtime, content: &str) -> Result<(), LoadFileError> {
    for (line, source) in content.lines().enumerate() {
        if source.is_empty() || source.trim().is_empty() {
            continue;
        }

        let toks = tokenising::tokenise(&source);
        let item = match parsing::Parser::parse_line(&toks) {
            Ok(item) => item,
            Err(err) => {
                let msg = format!("on line {}: {}", line, err);
                return Err(LoadFileError::ParseError(msg));
            }
        };

        match rt.eval_line_item(item)? {
            EvalResult::Empty => {}
            EvalResult::Value(_) => {}
            EvalResult::PrintValue(expr, val) => {
                let range = expr.fc();
                let input_slice = &source[range.start..range.end];

                println!(
                    "{} => {:<20} [{}]",
                    input_slice,
                    val.kind.to_string(),
                    val.unit
                );
            }
        }
    }
    Ok(())
}

fn repl(rt: &mut runtime::Runtime) -> Result<(), Box<dyn std::error::Error>> {
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
                let line_item = parsing::Parser::parse_line(&toks);

                match line_item {
                    Ok(item) => match rt.eval_line_item(item) {
                        Ok(EvalResult::Empty) => {}
                        Ok(EvalResult::Value(val)) => {
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

                            let units = rt.find_unit_likes(&val.unit);
                            let mut matches = closest_match(&val.kind, units);

                            // Iterator speghetti!!
                            // Try to take the best candidate, if it's actually anything
                            // unique (not `s` vs second)
                            let candidates = matches
                                .next()
                                .into_iter()
                                .filter(|(_, _, val_kind)| val_kind != &val.kind)
                                .map(|(name, pre, val_kind)| {
                                    (val_kind, name_with_prefix(&name, pre))
                                })
                                // Then take the "raw" unit, this one is always printed
                                // as a kind of fallback in case the suggestion is not good.
                                .chain(Some((val.kind.clone(), val.unit.to_string())).into_iter())
                                // Then the rest of the candidates (that are unique)
                                .chain(
                                    matches
                                        .filter(|(_, _, val_kind)| val_kind != &val.kind)
                                        .map(|(name, pre, val_kind)| {
                                            (val_kind, name_with_prefix(&name, pre))
                                        }),
                                )
                                .take(3);

                            for (i, (val, unit)) in candidates.enumerate() {
                                let marker = if i == 0 {
                                    format!("{}", "✔".green())
                                } else {
                                    format!("{}", "⇒".bright_black())
                                };

                                let response = format!("{:<12} [{}]", val.to_string(), unit);
                                println!("{} {}", marker, response.bright_black());
                            }
                        }
                        Ok(EvalResult::PrintValue(expr, val)) => {
                            let range = expr.fc();
                            let input_slice = &line[range.start..range.end];

                            let value_part = format!("{} => {}", input_slice, val.kind);
                            let response = format!("{:<12} [{}]", value_part, val.unit);
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();

    let mut rt = Runtime::new();
    load_file(&mut rt, include_str!("../lib/prelude.nat"))?;

    if let Some(file_path) = args.first() {
        let content = std::fs::read_to_string(file_path)?;
        load_file(&mut rt, &content)?;
    }

    repl(&mut rt)
}

type Rating = u64;

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
