use runtime::EvalResult;
use rustyline::error::ReadlineError;

use owo_colors::OwoColorize;

pub mod syntax;

pub mod parsing;
pub mod tokenising;

pub mod term;
pub mod runtime;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut rl = rustyline::Editor::<()>::new();

    let mut rt = runtime::Runtime::new();

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
                    Ok(item) => {
                        match rt.eval_line_item(item) {
                            Ok(EvalResult::Empty) => {},
                            Ok(EvalResult::Value(val)) => {
                                let response = format!("{:<20} [{}]", val.kind.to_string(), val.unit);
                                println!("{} {}", "✔".green(), response.bright_black());
                            },
                            Ok(EvalResult::PrintValue(expr, val)) => {
                                let response = format!("{:?} => {:<20} [{}]", expr, val.kind.to_string(), val.unit);
                                println!("{} {}", "✔".green(), response.bright_black());
                            },
                            Err(err) => {
                                eprintln!("{} {}", "✘".red(), err);
                            }
                        }
                    }
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
