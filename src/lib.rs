use std::collections::HashMap;
use std::fmt;
use std::thread;

mod number;

use number::{parse_number, Number};

const EVAL_STACK_SIZE_BYTES: usize = 1024 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Number(Number),
    Symbol(String),
    List(Vec<Expr>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Number(Number),
    Bool(bool),
    Atom(String),
    Lambda(Lambda),
    Nil,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Lambda {
    params: Vec<String>,
    body: Expr,
    captured: HashMap<String, Value>,
    self_name: Option<String>,
    public_arity: usize,
    auto_acc_init: Option<Number>,
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(n) => {
                write!(f, "{n}")
            }
            Self::Bool(v) => write!(f, "{v}"),
            Self::Atom(atom) => write!(f, ":{atom}"),
            Self::Lambda(_) => write!(f, "<fn>"),
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
    let src = src.to_string();
    eval_with_large_stack(env, move |env| {
        let expr = parse(&src)?;
        eval(&expr, env)
    })
}

pub fn eval_program(src: &str, env: &mut Env) -> Result<Value, LispError> {
    let src = src.to_string();
    eval_with_large_stack(env, move |env| {
        let exprs = parse_program(&src)?;
        let mut last = Value::Nil;
        for expr in exprs {
            last = eval(&expr, env)?;
        }
        Ok(last)
    })
}

fn eval_with_large_stack<F>(env: &mut Env, eval_fn: F) -> Result<Value, LispError>
where
    F: FnOnce(&mut Env) -> Result<Value, LispError> + Send + 'static,
{
    let mut moved_env = std::mem::take(env);
    let handle = thread::Builder::new()
        .stack_size(EVAL_STACK_SIZE_BYTES)
        .spawn(move || {
            let result = eval_fn(&mut moved_env);
            (moved_env, result)
        })
        .map_err(|err| LispError::Eval(format!("failed to start evaluation thread: {err}")))?;

    let (new_env, result) = handle
        .join()
        .map_err(|_| LispError::Eval("evaluation thread panicked".into()))?;
    *env = new_env;
    result
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

pub fn parse_program(src: &str) -> Result<Vec<Expr>, LispError> {
    let tokens = tokenize(src);
    if tokens.is_empty() {
        return Ok(Vec::new());
    }

    let mut exprs = Vec::new();
    let mut pos = 0;
    while pos < tokens.len() {
        let (expr, next) = parse_expr(&tokens, pos)?;
        exprs.push(expr);
        pos = next;
    }

    Ok(exprs)
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
    } else if let Some(n) = parse_number(token) {
        Ok((Expr::Number(n), pos + 1))
    } else {
        Ok((Expr::Symbol(token.clone()), pos + 1))
    }
}

pub fn eval(expr: &Expr, env: &mut Env) -> Result<Value, LispError> {
    match eval_with_tail(expr, env, None, true)? {
        EvalControl::Value(value) => Ok(value),
        EvalControl::TailCall(_) => {
            Err(LispError::Eval("tail call escaped function context".into()))
        }
    }
}

fn eval_value(expr: &Expr, env: &mut Env, self_name: Option<&str>) -> Result<Value, LispError> {
    match eval_with_tail(expr, env, self_name, false)? {
        EvalControl::Value(value) => Ok(value),
        EvalControl::TailCall(_) => {
            Err(LispError::Eval("tail call escaped function context".into()))
        }
    }
}

enum EvalControl {
    Value(Value),
    TailCall(Vec<Value>),
}

fn eval_with_tail(
    expr: &Expr,
    env: &mut Env,
    self_name: Option<&str>,
    in_tail_position: bool,
) -> Result<EvalControl, LispError> {
    match expr {
        Expr::Number(n) => Ok(EvalControl::Value(Value::Number(n.clone()))),
        Expr::Symbol(name) => match name.as_str() {
            "true" => Ok(EvalControl::Value(Value::Bool(true))),
            "false" => Ok(EvalControl::Value(Value::Bool(false))),
            "nil" => Ok(EvalControl::Value(Value::Nil)),
            _ if name.starts_with(':') && name.len() > 1 => Ok(EvalControl::Value(Value::Atom(
                name.trim_start_matches(':').to_string(),
            ))),
            _ => env
                .get(name)
                .cloned()
                .map(EvalControl::Value)
                .ok_or_else(|| LispError::Eval(format!("undefined symbol: {name}"))),
        },
        Expr::List(list) => eval_list(list, env, self_name, in_tail_position),
    }
}

fn eval_list(
    list: &[Expr],
    env: &mut Env,
    self_name: Option<&str>,
    in_tail_position: bool,
) -> Result<EvalControl, LispError> {
    let Some((head, tail)) = list.split_first() else {
        return Ok(EvalControl::Value(Value::Nil));
    };

    if in_tail_position {
        if let (Some(name), Expr::Symbol(head_name)) = (self_name, head) {
            if head_name == name {
                let mut args = Vec::with_capacity(tail.len());
                for expr in tail {
                    args.push(eval_value(expr, env, self_name)?);
                }
                return Ok(EvalControl::TailCall(args));
            }
        }
    }

    if let Expr::Symbol(op) = head {
        match op.as_str() {
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
                let value = eval_value(&tail[1], env, self_name)?;
                env.define(key, value.clone())?;
                Ok(EvalControl::Value(value))
            }
            "+" | "-" | "*" | "/" => eval_arithmetic(op, tail, env, self_name),
            "<" => eval_less_than(tail, env, self_name),
            "if" => {
                if !(2..=3).contains(&tail.len()) {
                    return Err(LispError::Eval("if expects 2 or 3 arguments".into()));
                }

                let condition = eval_value(&tail[0], env, self_name)?;
                if truthy(&condition) {
                    eval_with_tail(&tail[1], env, self_name, in_tail_position)
                } else if tail.len() == 3 {
                    eval_with_tail(&tail[2], env, self_name, in_tail_position)
                } else {
                    Ok(EvalControl::Value(Value::Nil))
                }
            }
            "|>" => eval_pipe(tail, env, self_name),
            "fn" => eval_fn(tail, env).map(EvalControl::Value),
            "print" => {
                if tail.len() != 1 {
                    return Err(LispError::Eval("print expects exactly 1 argument".into()));
                }
                let value = eval_value(&tail[0], env, self_name)?;
                println!("{value}");
                Ok(EvalControl::Value(value))
            }
            _ => {
                let callee = eval_value(head, env, self_name)?;
                let mut args = Vec::with_capacity(tail.len());
                for expr in tail {
                    args.push(eval_value(expr, env, self_name)?);
                }
                apply_function(callee, args).map(EvalControl::Value)
            }
        }
    } else {
        let callee = eval_value(head, env, self_name)?;
        let mut args = Vec::with_capacity(tail.len());
        for expr in tail {
            args.push(eval_value(expr, env, self_name)?);
        }
        apply_function(callee, args).map(EvalControl::Value)
    }
}

fn eval_fn(args: &[Expr], env: &mut Env) -> Result<Value, LispError> {
    let form_name = "fn";
    if args.len() != 2 {
        return Err(LispError::Eval(format!(
            "{form_name} expects exactly 2 arguments"
        )));
    }
    let params_expr = &args[0];
    let body = args[1].clone();

    let Expr::List(items) = params_expr else {
        return Err(LispError::Eval(format!(
            "{form_name} first argument must be a parameter list"
        )));
    };
    if items.is_empty() {
        return Err(LispError::Eval(format!(
            "{form_name} parameter list must include at least one symbol"
        )));
    }

    let mut names = Vec::with_capacity(items.len());
    for item in items {
        let Expr::Symbol(name) = item else {
            return Err(LispError::Eval(format!(
                "{form_name} parameter list must contain only symbols"
            )));
        };
        names.push(name.clone());
    }

    let recursive = names.first().is_some_and(|name| name == "self") && names.len() >= 2;

    let (self_name, params) = if recursive {
        let self_name = names[0].clone();
        (Some(self_name), names[1..].to_vec())
    } else {
        (None, names)
    };

    let public_arity = params.len();
    let (params, body, auto_acc_init) = if recursive {
        auto_tail_transform_fn(params, body, self_name.as_deref())
    } else {
        (params, body, None)
    };

    Ok(Value::Lambda(Lambda {
        params,
        body,
        captured: env.vars.clone(),
        self_name,
        public_arity,
        auto_acc_init,
    }))
}

fn auto_tail_transform_fn(
    params: Vec<String>,
    body: Expr,
    self_name: Option<&str>,
) -> (Vec<String>, Expr, Option<Number>) {
    let Some(self_name) = self_name else {
        return (params, body, None);
    };
    if params.len() != 1 {
        return (params, body, None);
    }
    let n = params[0].clone();

    let Expr::List(items) = &body else {
        return (params, body, None);
    };
    if items.len() != 4 {
        return (params, body, None);
    }
    if !matches!(&items[0], Expr::Symbol(op) if op == "if") {
        return (params, body, None);
    }

    let cond = items[1].clone();
    let base = items[2].clone();
    let rec_case = items[3].clone();

    let Expr::List(rec_items) = rec_case else {
        return (params, body, None);
    };
    if rec_items.len() != 3 {
        return (params, body, None);
    }

    let op = match &rec_items[0] {
        Expr::Symbol(op) if op == "+" || op == "*" => op.clone(),
        _ => return (params, body, None),
    };
    let acc_init = if op == "+" {
        Number::from_i64(0)
    } else {
        Number::from_i64(1)
    };

    let (step_expr, next_n) = match (&rec_items[1], &rec_items[2]) {
        (left, Expr::List(call)) if is_self_one_arg_call(call, self_name) => {
            (left.clone(), call[1].clone())
        }
        (Expr::List(call), right) if is_self_one_arg_call(call, self_name) => {
            (right.clone(), call[1].clone())
        }
        _ => return (params, body, None),
    };

    let acc_name = "__fnr_acc";
    let acc = Expr::Symbol(acc_name.into());
    let acc_base = Expr::List(vec![Expr::Symbol(op.clone()), base, acc.clone()]);
    let next_acc = Expr::List(vec![Expr::Symbol(op), acc.clone(), step_expr]);
    let recur = Expr::List(vec![Expr::Symbol(self_name.into()), next_n, next_acc]);

    let new_body = Expr::List(vec![Expr::Symbol("if".into()), cond, acc_base, recur]);
    let mut new_params = vec![n];
    new_params.push(acc_name.into());
    (new_params, new_body, Some(acc_init))
}

fn is_self_one_arg_call(items: &[Expr], self_name: &str) -> bool {
    items.len() == 2 && matches!(&items[0], Expr::Symbol(name) if name == self_name)
}

fn apply_function(callee: Value, args: Vec<Value>) -> Result<Value, LispError> {
    let Value::Lambda(lambda) = callee.clone() else {
        return Err(LispError::Eval(
            "first item in list must evaluate to a function".into(),
        ));
    };

    if lambda.public_arity != args.len() {
        return Err(LispError::Eval(format!(
            "function expected {} arguments, got {}",
            lambda.public_arity,
            args.len()
        )));
    }

    let mut current_args = args;
    if let Some(init) = lambda.auto_acc_init {
        current_args.push(Value::Number(init));
    }
    loop {
        let mut call_env = Env {
            vars: lambda.captured.clone(),
        };

        if let Some(self_name) = &lambda.self_name {
            call_env.vars.insert(self_name.clone(), callee.clone());
        }

        for (name, value) in lambda.params.iter().zip(current_args) {
            call_env.vars.insert(name.clone(), value);
        }

        match eval_with_tail(
            &lambda.body,
            &mut call_env,
            lambda.self_name.as_deref(),
            true,
        )? {
            EvalControl::Value(value) => return Ok(value),
            EvalControl::TailCall(next_args) => {
                if lambda.params.len() != next_args.len() {
                    return Err(LispError::Eval(format!(
                        "function expected {} arguments, got {}",
                        lambda.params.len(),
                        next_args.len()
                    )));
                }
                current_args = next_args;
            }
        }
    }
}

fn eval_arithmetic(
    op: &str,
    args: &[Expr],
    env: &mut Env,
    self_name: Option<&str>,
) -> Result<EvalControl, LispError> {
    if args.is_empty() {
        return Err(LispError::Eval(format!(
            "{op} expects at least one argument"
        )));
    }

    let mut numbers = Vec::with_capacity(args.len());
    for expr in args {
        let value = eval_value(expr, env, self_name)?;
        match value {
            Value::Number(n) => numbers.push(n),
            Value::Nil | Value::Bool(_) | Value::Atom(_) | Value::Lambda(_) => {
                return Err(LispError::Eval(
                    "arithmetic only supports numeric values".into(),
                ))
            }
        }
    }

    let result = match op {
        "+" => numbers
            .into_iter()
            .fold(Number::from_i64(0), |acc, n| acc + n),
        "*" => numbers
            .into_iter()
            .fold(Number::from_i64(1), |acc, n| acc * n),
        "-" => {
            let first = numbers[0].clone();
            if numbers.len() == 1 {
                -first
            } else {
                first
                    - numbers[1..]
                        .iter()
                        .cloned()
                        .fold(Number::from_i64(0), |acc, n| acc + n)
            }
        }
        "/" => {
            let first = numbers[0].clone();
            if numbers.len() == 1 {
                if first.is_zero() {
                    return Err(LispError::Eval("division by zero".into()));
                }
                first.reciprocal().expect("first is non-zero")
            } else {
                let denom = numbers[1..]
                    .iter()
                    .cloned()
                    .fold(Number::from_i64(1), |acc, n| acc * n);
                if denom.is_zero() {
                    return Err(LispError::Eval("division by zero".into()));
                }
                first * denom.reciprocal().expect("denominator is non-zero")
            }
        }
        _ => unreachable!(),
    };

    Ok(EvalControl::Value(Value::Number(result)))
}

fn eval_less_than(
    args: &[Expr],
    env: &mut Env,
    self_name: Option<&str>,
) -> Result<EvalControl, LispError> {
    if args.len() != 2 {
        return Err(LispError::Eval("< expects exactly 2 arguments".into()));
    }
    let left = value_to_number(eval_value(&args[0], env, self_name)?)?;
    let right = value_to_number(eval_value(&args[1], env, self_name)?)?;
    Ok(EvalControl::Value(Value::Bool(left < right)))
}

fn eval_pipe(
    stages: &[Expr],
    env: &mut Env,
    self_name: Option<&str>,
) -> Result<EvalControl, LispError> {
    let Some((first, rest)) = stages.split_first() else {
        return Err(LispError::Eval("|> expects at least 1 argument".into()));
    };

    let mut current = eval_value(first, env, self_name)?;
    for stage in rest {
        current = apply_pipe_stage(stage, current, env, self_name)?;
    }

    Ok(EvalControl::Value(current))
}

fn apply_pipe_stage(
    stage: &Expr,
    input: Value,
    env: &mut Env,
    self_name: Option<&str>,
) -> Result<Value, LispError> {
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
                numbers.push(value_to_number(eval_value(expr, env, self_name)?)?);
            }

            let result = match op.as_str() {
                "+" => numbers
                    .into_iter()
                    .fold(Number::from_i64(0), |acc, n| acc + n),
                "*" => numbers
                    .into_iter()
                    .fold(Number::from_i64(1), |acc, n| acc * n),
                "-" => {
                    let first = numbers[0].clone();
                    if numbers.len() == 1 {
                        -first
                    } else {
                        first
                            - numbers[1..]
                                .iter()
                                .cloned()
                                .fold(Number::from_i64(0), |acc, n| acc + n)
                    }
                }
                "/" => {
                    let first = numbers[0].clone();
                    if numbers.len() == 1 {
                        if first.is_zero() {
                            return Err(LispError::Eval("division by zero".into()));
                        }
                        first.reciprocal().expect("first is non-zero")
                    } else {
                        let denom = numbers[1..]
                            .iter()
                            .cloned()
                            .fold(Number::from_i64(1), |acc, n| acc * n);
                        if denom.is_zero() {
                            return Err(LispError::Eval("division by zero".into()));
                        }
                        first * denom.reciprocal().expect("denominator is non-zero")
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

fn value_to_number(value: Value) -> Result<Number, LispError> {
    match value {
        Value::Number(n) => Ok(n),
        Value::Bool(_) | Value::Atom(_) | Value::Nil | Value::Lambda(_) => Err(LispError::Eval(
            "arithmetic only supports numeric values".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(v: i64) -> Number {
        Number::from_i64(v)
    }

    #[test]
    fn parses_nested_expression() {
        let parsed = parse("(+ 1 (* 2 3))").expect("parse should succeed");
        assert_eq!(
            parsed,
            Expr::List(vec![
                Expr::Symbol("+".into()),
                Expr::Number(n(1)),
                Expr::List(vec![
                    Expr::Symbol("*".into()),
                    Expr::Number(n(2)),
                    Expr::Number(n(3)),
                ])
            ])
        );
    }

    #[test]
    fn parses_multiple_top_level_expressions() {
        let parsed =
            parse_program("(define answer 42) (* answer 2)").expect("program parse should succeed");
        assert_eq!(parsed.len(), 2);
    }

    #[test]
    fn evals_define_and_lookup() {
        let mut env = Env::default();
        let value = eval_source("(define answer (+ 40 2))", &mut env).expect("define should work");
        assert_eq!(value, Value::Number(n(42)));

        let looked_up = eval_source("answer", &mut env).expect("symbol lookup should work");
        assert_eq!(looked_up, Value::Number(n(42)));
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
            Value::Number(n(10))
        );
        assert_eq!(
            eval_source("(- 10 3 2)", &mut env).expect("- should work"),
            Value::Number(n(5))
        );
        assert_eq!(
            eval_source("(/ 20 5)", &mut env).expect("/ should work"),
            Value::Number(n(4))
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
            Value::Number(n(10))
        );
        assert_eq!(
            eval_source("(if false 10 0)", &mut env).expect("if false should work"),
            Value::Number(n(0))
        );
    }

    #[test]
    fn evals_pipeline_expression() {
        let mut env = Env::default();
        assert_eq!(
            eval_source("(|> 5 (+ 3) (* 2))", &mut env).expect("pipeline should work"),
            Value::Number(n(16))
        );
    }

    #[test]
    fn evals_program_and_returns_last_expression() {
        let mut env = Env::default();
        let src = "(define answer (+ 40 2))\n(* answer 2)";
        assert_eq!(
            eval_program(src, &mut env).expect("program eval should work"),
            Value::Number(n(84))
        );
    }

    #[test]
    fn evals_anonymous_function() {
        let mut env = Env::default();
        assert_eq!(
            eval_source("((fn (x) (+ x 1)) 4)", &mut env).expect("anonymous function should work"),
            Value::Number(n(5))
        );
    }

    #[test]
    fn evals_anonymous_recursion_with_fn() {
        let mut env = Env::default();
        let src = "((fn (self n) (if (< n 2) 1 (* n (self (- n 1))))) 5)";
        assert_eq!(
            eval_source(src, &mut env).expect("anonymous recursion should work"),
            Value::Number(n(120))
        );
    }

    #[test]
    fn supports_deep_tail_recursion_with_fn() {
        let mut env = Env::default();
        let src = "((fn (self n acc) (if (< n 1) acc (self (- n 1) (+ acc 1)))) 20000 0)";
        assert_eq!(
            eval_source(src, &mut env).expect("deep tail recursion should work"),
            Value::Number(n(20000))
        );
    }

    #[test]
    fn supports_deep_non_tail_recursion_with_fn() {
        let mut env = Env::default();
        let src = "((fn (self n) (if (< n 1) 0 (+ 1 (self (- n 1))))) 5000)";
        assert_eq!(
            eval_source(src, &mut env).expect("deep non-tail recursion should work"),
            Value::Number(n(5000))
        );
    }

    #[test]
    fn auto_transforms_non_tail_recursion_for_large_depth() {
        let mut env = Env::default();
        let src = "((fn (self n) (if (< n 1) 0 (+ 1 (self (- n 1))))) 100000)";
        assert_eq!(
            eval_source(src, &mut env).expect("auto transformed non-tail recursion should work"),
            Value::Number(n(100000))
        );
    }

    #[test]
    fn parses_decimal_as_exact_rational() {
        let parsed = parse("3.14").expect("parse should succeed");
        assert_eq!(
            parsed,
            Expr::Number(parse_number("3.14").expect("number parse"))
        );
    }
}
