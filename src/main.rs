use runtime::{CallStack, EvalResult, Runtime};

use thiserror::Error;

use syntax::HasFC;

pub mod syntax;

pub mod parsing;
pub mod tokenising;

pub mod functions;
pub mod runtime;
pub mod value_unit;

pub mod num;
pub mod repl;

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();

    let mut rt = Runtime::new();
    load_file(&mut rt, include_str!("../lib/prelude.nat"), false)?;

    if let Some(file_path) = args.first() {
        let content = std::fs::read_to_string(file_path)?;
        load_file(&mut rt, &content, true)?;
    }

    repl::repl(&mut rt)
}
