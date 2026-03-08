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
            Self::Nil => write!(f, "nil"),
        }
    }
}

#[derive(Default)]
pub struct Env {
    vars: HashMap<String, Value>,
}

impl Env {
    pub fn set(&mut self, key: impl Into<String>, value: Value) {
        self.vars.insert(key.into(), value);
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
        Expr::Symbol(name) => env
            .get(name)
            .cloned()
            .ok_or_else(|| LispError::Eval(format!("undefined symbol: {name}"))),
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
            env.set(key, value.clone());
            Ok(value)
        }
        "+" | "-" | "*" | "/" => eval_arithmetic(op, tail, env),
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
            Value::Nil => {
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
}
