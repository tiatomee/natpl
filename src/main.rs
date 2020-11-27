use runtime::{EvalResult, Runtime};
use rustyline::error::ReadlineError;

use owo_colors::OwoColorize;
use thiserror::Error;

use syntax::HasFC;

pub mod syntax;

pub mod parsing;
pub mod tokenising;

pub mod runtime;
pub mod term;


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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut rl = rustyline::Editor::<()>::new();

    let mut rt = runtime::Runtime::new();

    load_file(&mut rt, include_str!("../lib/prelude.nat"))?;

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
                            let response = format!("{:<20} [{}]", val.kind.to_string(), val.unit);
                            println!("{} {}", "✔".green(), response.bright_black());
                        }
                        Ok(EvalResult::PrintValue(expr, val)) => {
                            let range = expr.fc();
                            let input_slice = &line[range.start..range.end];

                            let response = format!(
                                "{} => {:<20} [{}]",
                                input_slice,
                                val.kind.to_string(),
                                val.unit
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
