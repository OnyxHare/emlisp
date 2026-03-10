use std::collections::HashMap;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Number(f64),
    Symbol(String),
    List(Vec<Expr>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Number(f64),
    Bool(bool),
    Atom(String),
    Nil,
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(n) => {
                if n.fract() == 0.0 {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{n}")
                }
            }
            Self::Bool(v) => write!(f, "{v}"),
            Self::Atom(atom) => write!(f, ":{atom}"),
            Self::Nil => write!(f, "nil"),
        }
    }
}

#[derive(Default)]
pub struct Env {
    vars: HashMap<String, Value>,
}

impl Env {
    pub fn define(&mut self, key: impl Into<String>, value: Value) -> Result<(), LispError> {
        let key = key.into();
        if self.vars.contains_key(&key) {
            return Err(LispError::Eval(format!(
                "cannot redefine immutable binding: {key}"
            )));
        }
        self.vars.insert(key, value);
        Ok(())
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.vars.get(key)
    }
}

#[derive(Debug, PartialEq)]
pub enum LispError {
    Parse(String),
    Eval(String),
}

impl fmt::Display for LispError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(msg) => write!(f, "parse error: {msg}"),
            Self::Eval(msg) => write!(f, "eval error: {msg}"),
        }
    }
}

impl std::error::Error for LispError {}

pub fn eval_source(src: &str, env: &mut Env) -> Result<Value, LispError> {
    let expr = parse(src)?;
    eval(&expr, env)
}

pub fn parse(src: &str) -> Result<Expr, LispError> {
    let tokens = tokenize(src);
    if tokens.is_empty() {
        return Err(LispError::Parse("empty input".into()));
    }

    let (expr, next) = parse_expr(&tokens, 0)?;
    if next != tokens.len() {
        return Err(LispError::Parse(
            "unexpected tokens after expression".into(),
        ));
    }
    Ok(expr)
}

fn tokenize(src: &str) -> Vec<String> {
    src.replace('(', " ( ")
        .replace(')', " ) ")
        .split_whitespace()
        .map(ToOwned::to_owned)
        .collect()
}

fn parse_expr(tokens: &[String], pos: usize) -> Result<(Expr, usize), LispError> {
    let token = tokens
        .get(pos)
        .ok_or_else(|| LispError::Parse("unexpected end of input".into()))?;

    if token == "(" {
        let mut list = Vec::new();
        let mut i = pos + 1;

        while let Some(next) = tokens.get(i) {
            if next == ")" {
                return Ok((Expr::List(list), i + 1));
            }
            let (expr, next_i) = parse_expr(tokens, i)?;
            list.push(expr);
            i = next_i;
        }

        Err(LispError::Parse("missing closing ')'".into()))
    } else if token == ")" {
        Err(LispError::Parse("unexpected ')'".into()))
    } else if let Ok(n) = token.parse::<f64>() {
        Ok((Expr::Number(n), pos + 1))
    } else {
        Ok((Expr::Symbol(token.clone()), pos + 1))
    }
}

pub fn eval(expr: &Expr, env: &mut Env) -> Result<Value, LispError> {
    match expr {
        Expr::Number(n) => Ok(Value::Number(*n)),
        Expr::Symbol(name) => match name.as_str() {
            "true" => Ok(Value::Bool(true)),
            "false" => Ok(Value::Bool(false)),
            "nil" => Ok(Value::Nil),
            _ if name.starts_with(':') && name.len() > 1 => {
                Ok(Value::Atom(name.trim_start_matches(':').to_string()))
            }
            _ => env
                .get(name)
                .cloned()
                .ok_or_else(|| LispError::Eval(format!("undefined symbol: {name}"))),
        },
        Expr::List(list) => eval_list(list, env),
    }
}

fn eval_list(list: &[Expr], env: &mut Env) -> Result<Value, LispError> {
    let Some((head, tail)) = list.split_first() else {
        return Ok(Value::Nil);
    };

    let op = match head {
        Expr::Symbol(name) => name.as_str(),
        _ => {
            return Err(LispError::Eval(
                "first item in list must be a symbol".into(),
            ))
        }
    };

    match op {
        "define" => {
            if tail.len() != 2 {
                return Err(LispError::Eval("define expects exactly 2 arguments".into()));
            }
            let key = match &tail[0] {
                Expr::Symbol(name) => name.clone(),
                _ => {
                    return Err(LispError::Eval(
                        "define first argument must be a symbol".into(),
                    ))
                }
            };
            let value = eval(&tail[1], env)?;
            env.define(key, value.clone())?;
            Ok(value)
        }
        "+" | "-" | "*" | "/" => eval_arithmetic(op, tail, env),
        "if" => {
            if !(2..=3).contains(&tail.len()) {
                return Err(LispError::Eval("if expects 2 or 3 arguments".into()));
            }

            let condition = eval(&tail[0], env)?;
            if truthy(&condition) {
                eval(&tail[1], env)
            } else if tail.len() == 3 {
                eval(&tail[2], env)
            } else {
                Ok(Value::Nil)
            }
        }
        "|>" => eval_pipe(tail, env),
        "print" => {
            if tail.len() != 1 {
                return Err(LispError::Eval("print expects exactly 1 argument".into()));
            }
            let value = eval(&tail[0], env)?;
            println!("{value}");
            Ok(value)
        }
        _ => Err(LispError::Eval(format!("unknown function: {op}"))),
    }
}

fn eval_arithmetic(op: &str, args: &[Expr], env: &mut Env) -> Result<Value, LispError> {
    if args.is_empty() {
        return Err(LispError::Eval(format!(
            "{op} expects at least one argument"
        )));
    }

    let mut numbers = Vec::with_capacity(args.len());
    for expr in args {
        let value = eval(expr, env)?;
        match value {
            Value::Number(n) => numbers.push(n),
            Value::Nil | Value::Bool(_) | Value::Atom(_) => {
                return Err(LispError::Eval(
                    "arithmetic only supports numeric values".into(),
                ))
            }
        }
    }

    let result = match op {
        "+" => numbers.into_iter().sum(),
        "*" => numbers.into_iter().product(),
        "-" => {
            let first = numbers[0];
            if numbers.len() == 1 {
                -first
            } else {
                first - numbers[1..].iter().sum::<f64>()
            }
        }
        "/" => {
            let first = numbers[0];
            if numbers.len() == 1 {
                if first == 0.0 {
                    return Err(LispError::Eval("division by zero".into()));
                }
                1.0 / first
            } else {
                let denom = numbers[1..].iter().product::<f64>();
                if denom == 0.0 {
                    return Err(LispError::Eval("division by zero".into()));
                }
                first / denom
            }
        }
        _ => unreachable!(),
    };

    Ok(Value::Number(result))
}

fn eval_pipe(stages: &[Expr], env: &mut Env) -> Result<Value, LispError> {
    let Some((first, rest)) = stages.split_first() else {
        return Err(LispError::Eval("|> expects at least 1 argument".into()));
    };

    let mut current = eval(first, env)?;
    for stage in rest {
        current = apply_pipe_stage(stage, current, env)?;
    }

    Ok(current)
}

fn apply_pipe_stage(stage: &Expr, input: Value, env: &mut Env) -> Result<Value, LispError> {
    let Expr::List(items) = stage else {
        return Err(LispError::Eval(
            "pipeline stage must be a list, e.g. (+ 1)".into(),
        ));
    };
    let Some((head, tail)) = items.split_first() else {
        return Err(LispError::Eval("pipeline stage cannot be empty".into()));
    };
    let Expr::Symbol(op) = head else {
        return Err(LispError::Eval(
            "pipeline stage must start with a symbol".into(),
        ));
    };

    match op.as_str() {
        "+" | "-" | "*" | "/" => {
            let mut numbers = Vec::with_capacity(tail.len() + 1);
            numbers.push(value_to_number(input)?);
            for expr in tail {
                numbers.push(value_to_number(eval(expr, env)?)?);
            }

            let result = match op.as_str() {
                "+" => numbers.into_iter().sum(),
                "*" => numbers.into_iter().product(),
                "-" => {
                    let first = numbers[0];
                    if numbers.len() == 1 {
                        -first
                    } else {
                        first - numbers[1..].iter().sum::<f64>()
                    }
                }
                "/" => {
                    let first = numbers[0];
                    if numbers.len() == 1 {
                        if first == 0.0 {
                            return Err(LispError::Eval("division by zero".into()));
                        }
                        1.0 / first
                    } else {
                        let denom = numbers[1..].iter().product::<f64>();
                        if denom == 0.0 {
                            return Err(LispError::Eval("division by zero".into()));
                        }
                        first / denom
                    }
                }
                _ => unreachable!(),
            };

            Ok(Value::Number(result))
        }
        "print" => {
            if !tail.is_empty() {
                return Err(LispError::Eval(
                    "print in pipeline does not take explicit arguments".into(),
                ));
            }
            println!("{input}");
            Ok(input)
        }
        _ => Err(LispError::Eval(format!(
            "unknown function in pipeline stage: {op}"
        ))),
    }
}

fn truthy(value: &Value) -> bool {
    !matches!(value, Value::Bool(false) | Value::Nil)
}

fn value_to_number(value: Value) -> Result<f64, LispError> {
    match value {
        Value::Number(n) => Ok(n),
        Value::Bool(_) | Value::Atom(_) | Value::Nil => Err(LispError::Eval(
            "arithmetic only supports numeric values".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_expression() {
        let parsed = parse("(+ 1 (* 2 3))").expect("parse should succeed");
        assert_eq!(
            parsed,
            Expr::List(vec![
                Expr::Symbol("+".into()),
                Expr::Number(1.0),
                Expr::List(vec![
                    Expr::Symbol("*".into()),
                    Expr::Number(2.0),
                    Expr::Number(3.0),
                ])
            ])
        );
    }

    #[test]
    fn evals_define_and_lookup() {
        let mut env = Env::default();
        let value = eval_source("(define answer (+ 40 2))", &mut env).expect("define should work");
        assert_eq!(value, Value::Number(42.0));

        let looked_up = eval_source("answer", &mut env).expect("symbol lookup should work");
        assert_eq!(looked_up, Value::Number(42.0));
    }

    #[test]
    fn define_is_immutable() {
        let mut env = Env::default();
        eval_source("(define answer 42)", &mut env).expect("first define should work");

        let err = eval_source("(define answer 7)", &mut env).expect_err("redefine should fail");
        assert_eq!(
            err,
            LispError::Eval("cannot redefine immutable binding: answer".into())
        );
    }

    #[test]
    fn evals_arithmetic() {
        let mut env = Env::default();
        assert_eq!(
            eval_source("(+ 1 2 3 4)", &mut env).expect("+ should work"),
            Value::Number(10.0)
        );
        assert_eq!(
            eval_source("(- 10 3 2)", &mut env).expect("- should work"),
            Value::Number(5.0)
        );
        assert_eq!(
            eval_source("(/ 20 5)", &mut env).expect("/ should work"),
            Value::Number(4.0)
        );
    }

    #[test]
    fn evals_atom_and_bool_literals() {
        let mut env = Env::default();
        assert_eq!(
            eval_source(":ok", &mut env).expect("atom literal should work"),
            Value::Atom("ok".into())
        );
        assert_eq!(
            eval_source("true", &mut env).expect("true literal should work"),
            Value::Bool(true)
        );
    }

    #[test]
    fn evals_if_expression() {
        let mut env = Env::default();
        assert_eq!(
            eval_source("(if true 10 0)", &mut env).expect("if true should work"),
            Value::Number(10.0)
        );
        assert_eq!(
            eval_source("(if false 10 0)", &mut env).expect("if false should work"),
            Value::Number(0.0)
        );
    }

    #[test]
    fn evals_pipeline_expression() {
        let mut env = Env::default();
        assert_eq!(
            eval_source("(|> 5 (+ 3) (* 2))", &mut env).expect("pipeline should work"),
            Value::Number(16.0)
        );
    }
}
