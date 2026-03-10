use std::fs;
use std::io::{self, Write};

use emlisp::{eval_program, eval_source, Env};

fn load_file(path: &str, env: &mut Env) {
    match fs::read_to_string(path) {
        Ok(src) => match eval_program(&src, env) {
            Ok(value) => println!("=> {value}"),
            Err(err) => eprintln!("{err}"),
        },
        Err(err) => eprintln!("failed to read file: {err}"),
    }
}

fn main() {
    let file_to_load = std::env::args().nth(1);
    let mut env = Env::default();

    if let Some(path) = file_to_load {
        load_file(&path, &mut env);
    }

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

                if let Some(path) = input.strip_prefix(":load ") {
                    load_file(path.trim(), &mut env);
                    continue;
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
