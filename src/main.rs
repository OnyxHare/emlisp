use std::io::{self, Write};

use emlisp::{eval_source, Env};

fn main() {
    let mut env = Env::default();

    println!("emlisp REPL");
    println!("type :quit to exit");

    loop {
        print!("> ");
        if io::stdout().flush().is_err() {
            eprintln!("failed to flush stdout");
            break;
        }

        let mut line = String::new();
        match io::stdin().read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                let input = line.trim();
                if input.is_empty() {
                    continue;
                }
                if input == ":quit" || input == ":q" {
                    break;
                }

                match eval_source(input, &mut env) {
                    Ok(value) => println!("=> {value}"),
                    Err(err) => eprintln!("{err}"),
                }
            }
            Err(err) => {
                eprintln!("failed to read line: {err}");
                break;
            }
        }
    }
}
