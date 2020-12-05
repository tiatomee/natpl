use natpl_repl::ReadEvalResult;
use owo_colors::OwoColorize;
use rustyline::error::ReadlineError;
use thiserror::Error;

use natpl::{
    parsing,
    runtime::{self, CallStack, EvalResult, Runtime},
    syntax::HasFC,
    tokenising,
};

#[derive(Debug, Error)]
pub enum LoadFileError {
    #[error("Runtime error: {}", .0)]
    RuntimeError(#[from] runtime::ItemError),
    #[error("Parse error {}", .0)]
    ParseError(String),
}

/// Load a file into the runtime line-by line.
///
/// If `print` is true then printed-expressions are printed to stdout.
pub fn load_file(rt: &mut Runtime, content: &str, print: bool) -> Result<(), LoadFileError> {
    for (line, source) in content.lines().enumerate() {
        let toks = tokenising::tokenise(&source);
        let item = match parsing::Parser::parse_line(&toks) {
            Ok(item) => item,
            Err(err) => {
                let msg = format!("on line {}: {}", line, err);
                return Err(LoadFileError::ParseError(msg));
            }
        };

        match rt.eval_line_item(item, &mut CallStack::default())? {
            EvalResult::Empty => {}
            EvalResult::Value(_) => {}
            EvalResult::PrintValue(expr, val) => {
                if print {
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
            EvalResult::VariableSearchResult { .. } => {}
            EvalResult::UnitSearchResult { .. } => {}
        }
    }
    Ok(())
}

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

                match natpl_repl::read_eval(rt, &line) {
                    ReadEvalResult::ParseError(err) => {
                        eprintln!("Parse error: {}", err);
                    }
                    ReadEvalResult::ItemError(err) => {
                        eprintln!("{} {}", "✘".red(), err);
                    }
                    ReadEvalResult::Empty => {}
                    ReadEvalResult::SilentValue {
                        value: _,
                        display_candidates,
                    } => {
                        for (i, (unit, val)) in display_candidates.iter().take(3).enumerate() {
                            let marker = if i == 0 {
                                format!("{}", "✔".green())
                            } else {
                                format!("{}", "⇒".bright_black())
                            };

                            let response =
                                format!("{:expr_pad$} [{}]", val.to_string(), unit, expr_pad = 12);
                            println!("{} {}", marker, response.bright_black());
                        }
                    }
                    ReadEvalResult::PrintValue {
                        expr,
                        value: _,
                        display_candidates,
                    } => {
                        let range = expr.fc();
                        let input_slice = &line[range.start..range.end];

                        let value_part = format!("{} => {}", input_slice, display_candidates[0].1);
                        let response = format!(
                            "{:expr_pad$} [{}]",
                            value_part,
                            display_candidates[0].0,
                            expr_pad = EXPR_PAD
                        );
                        println!("{} {}", "✔".green(), response.bright_black());
                    }
                    ReadEvalResult::VariableSearchResult {
                        unit_aliases: _,
                        variables,
                        functions: _,
                    } => {
                        let longest_name = variables.keys().max_by(|a, b| a.len().cmp(&b.len()));
                        let name_length = longest_name.map(|s| s.len()).unwrap_or(0);

                        for (i, (name, display_candidates)) in variables.into_iter().enumerate() {
                            let marker = if i == 0 {
                                format!("{}", "✔".green())
                            } else {
                                format!("{}", "⇒".bright_black())
                            };

                            let (unit_name, val_kind) = display_candidates[0].clone();
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
                    ReadEvalResult::UnitSearchResult {
                        unit: _,
                        unit_aliases: _,
                        variables,
                    } => {
                        let longest_name = variables.keys().max_by(|a, b| a.len().cmp(&b.len()));
                        let name_length = longest_name.map(|s| s.len()).unwrap_or(0);

                        for (i, (name, display_candidates)) in variables.into_iter().enumerate() {
                            let marker = if i == 0 {
                                format!("{}", "✔".green())
                            } else {
                                format!("{}", "⇒".bright_black())
                            };

                            let (unit_name, val_kind) = display_candidates[0].clone();
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

    load_file(&mut rt, include_str!("../../lib/prelude.nat"), false)?;

    if let Some(file_path) = args.first() {
        let content = std::fs::read_to_string(file_path)?;
        load_file(&mut rt, &content, true)?;
    }

    repl(&mut rt)
}
