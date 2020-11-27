use rustyline::error::ReadlineError;

pub mod syntax;

pub mod parsing;
pub mod tokenising;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut rl = rustyline::Editor::<()>::new();

    loop {
        let readline = rl.readline("> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(&line);

                let toks = tokenising::tokenise(&line);
                let line_item = parsing::Parser::parse_line(&toks);

                match line_item {
                    Ok(item) => {
                        dbg!(item);
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
