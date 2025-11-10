#![allow(
    clippy::large_enum_variant,
    clippy::redundant_closure,
    clippy::wrong_self_convention,
    clippy::is_digit_ascii_radix,
    clippy::match_like_matches_macro,
    clippy::type_complexity,
    clippy::bool_comparison,
    clippy::len_zero,
    clippy::needless_borrow,
    clippy::single_match,
    clippy::needless_return,
    clippy::into_iter_on_ref,
    dead_code
)]

use anyhow::{anyhow, Result};
use regex::Regex;
use std::collections::HashMap;
use std::fmt;

use crate::db::models::EventVersion;

/// Represents an S-expression in our Lisp-like DSL
#[derive(Clone)]
pub enum SExpr {
    Nil,
    Symbol(String),
    String(String),
    Number(f64),
    Bool(bool),
    List(Vec<SExpr>),
    Lambda(Vec<String>, Box<SExpr>),
    Regex(Regex),
    Event(EventVersion),
    // A NativeFunction receives as the first argument the name with which it was called, if available, and receives evaluated arguments
    NativeFunction(String, fn(&str, Vec<SExpr>) -> Result<SExpr>),
    // A NativeForm receives the unevaluated arguments
    NativeForm(String, fn(&str, Vec<SExpr>, &Env) -> Result<SExpr>),
    DateTimeUTC(chrono::DateTime<chrono::Utc>),
}
impl SExpr {
    fn from_option_str(str: Option<String>) -> SExpr {
        str.map(|s| SExpr::String(s)).unwrap_or(SExpr::Nil)
    }
    fn from_option_datetime(dt: Option<chrono::DateTime<chrono::Utc>>) -> SExpr {
        dt.map(|d| SExpr::DateTimeUTC(d)).unwrap_or(SExpr::Nil)
    }

    fn into_event(&self) -> Option<&EventVersion> {
        match self {
            SExpr::Event(event) => Some(event),
            _ => None,
        }
    }

    fn into_list(&self) -> Option<Vec<SExpr>> {
        match self {
            SExpr::List(l) => Some(l.clone()),
            _ => None,
        }
    }

    fn is_true(&self) -> bool {
        match self {
            SExpr::Bool(b) => *b,
            _ => false,
        }
    }
}

impl fmt::Debug for SExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SExpr::Nil => write!(f, "Nil"),
            SExpr::Symbol(s) => write!(f, "Symbol({s})"),
            SExpr::String(s) => write!(f, "String({s})"),
            SExpr::Number(n) => write!(f, "Number({n})"),
            SExpr::Bool(b) => write!(f, "Bool({b})"),
            SExpr::List(l) => f.debug_list().entries(l).finish(),
            SExpr::Lambda(params, body) => write!(f, "Lambda({params:?}, {body:?})"),
            SExpr::Regex(r) => write!(f, "Regex({r})"),
            SExpr::Event(e) => write!(f, "Event({e:?})"),
            SExpr::NativeFunction(name, _) => write!(f, "NativeFunction({name})"),
            SExpr::NativeForm(name, _) => write!(f, "NativeForm({name})"),
            SExpr::DateTimeUTC(dt) => write!(f, "DateTimeUTC({dt:?})"),
        }
    }
}

/// Environment for storing variables and functions
#[derive(Debug, Clone)]
pub struct Env {
    vars: HashMap<String, SExpr>,
    outer: Option<Box<Env>>,
}

impl Env {
    pub fn new() -> Self {
        let mut env = Env {
            vars: HashMap::new(),
            outer: None,
        };
        env.setup_standard_lib();
        env
    }

    pub fn with_outer(outer: Env) -> Self {
        Env {
            vars: HashMap::new(),
            outer: Some(Box::new(outer)),
        }
    }

    fn setup_standard_lib(&mut self) {
        self.internal_set("true", SExpr::Bool(true));
        self.internal_set("false", SExpr::Bool(false));

        // Standard library functions
        // Comparison and arithmetic operators
        self.internal_set_native("=", native_comparison);
        self.internal_set_native("<", native_comparison);
        self.internal_set_native(">", native_comparison);
        self.internal_set_native("and", native_comparison);
        self.internal_set_native("or", native_comparison);
        self.internal_set_native("not", native_comparison);

        self.internal_set_native("+", native_arithmetic);
        self.internal_set_native("-", native_arithmetic);
        self.internal_set_native("*", native_arithmetic);
        self.internal_set_native("/", native_arithmetic);

        self.internal_set_native_form("lambda", native_lambda);
        self.internal_set_native_form("quote", native_quote);
        self.internal_set_native_form("funcall", native_funcall);
        self.internal_set_native_form("let", native_let);
        self.internal_set_native_form("eval", native_eval);

        // String operations
        self.internal_set_native("contains", native_contains);

        // Event property accessors
        self.internal_set_native("summary", native_event_field);
        self.internal_set_native("description", native_event_field);
        self.internal_set_native("location", native_event_field);
        self.internal_set_native("dtstart", native_event_field);
        self.internal_set_native("dtend", native_event_field);
        self.internal_set_native("uid", native_event_field);

        // Event filtering
        self.internal_set_native("by-event", native_byevent_runner);
    }

    pub fn get(&self, key: &str) -> Option<SExpr> {
        match self.vars.get(key) {
            Some(val) => Some(val.clone()),
            None => match &self.outer {
                Some(outer) => outer.get(key),
                None => None,
            },
        }
    }

    pub fn set(&mut self, key: String, val: SExpr) {
        self.vars.insert(key, val);
    }

    #[allow(unused)]
    fn internal_set_parse(&mut self, key: &str, val: &str) {
        match parse(val) {
            Ok(expr) => self.set(key.to_string(), expr),
            Err(e) => {
                eprintln!("Error setting variable {key}: {e}");
            }
        }
    }
    fn internal_set(&mut self, key: &str, val: SExpr) {
        self.set(key.to_string(), val);
    }

    fn internal_set_native(&mut self, key: &str, func: fn(&str, Vec<SExpr>) -> Result<SExpr>) {
        self.internal_set(key, SExpr::NativeFunction(key.to_string(), func));
    }

    fn internal_set_native_form(
        &mut self,
        key: &str,
        func: fn(&str, Vec<SExpr>, &Env) -> Result<SExpr>,
    ) {
        self.internal_set(key, SExpr::NativeForm(key.to_string(), func));
    }

    fn with_vars(&self, params: Vec<String>, param_values: Vec<SExpr>) -> Env {
        let mut env = Env::with_outer(self.clone());
        for (param, val) in params.iter().zip(param_values.iter()) {
            env.set(param.clone(), val.clone());
        }
        env
    }
}

/// Parse a string into an S-expression
pub fn parse(input: &str) -> Result<SExpr> {
    let mut chars = input.chars().peekable();
    parse_expr(&mut chars)
}

fn parse_expr<I>(chars: &mut std::iter::Peekable<I>) -> Result<SExpr>
where
    I: Iterator<Item = char>,
{
    skip_whitespace(chars);

    match chars.peek() {
        Some('(') => {
            chars.next(); // consume '('
            let mut list = Vec::new();

            loop {
                skip_whitespace(chars);
                match chars.peek() {
                    Some(')') => {
                        chars.next(); // consume ')'
                        return Ok(SExpr::List(list));
                    }
                    Some(_) => list.push(parse_expr(chars)?),
                    None => return Err(anyhow!("Unexpected end of input")),
                }
            }
        }
        Some('"') => parse_string(chars),
        Some(c) if c.is_digit(10) || *c == '-' => parse_number(chars),
        Some(c) if c.is_alphabetic() || is_symbol_char(*c) => parse_symbol(chars),
        Some(c) => Err(anyhow!("Unexpected character: {c}")),
        None => Err(anyhow!("Unexpected end of input")),
    }
}

fn skip_whitespace<I>(chars: &mut std::iter::Peekable<I>)
where
    I: Iterator<Item = char>,
{
    while let Some(c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else {
            break;
        }
    }
}

fn parse_string<I>(chars: &mut std::iter::Peekable<I>) -> Result<SExpr>
where
    I: Iterator<Item = char>,
{
    chars.next(); // consume opening quote
    let mut s = String::new();

    loop {
        match chars.next() {
            Some('"') => return Ok(SExpr::String(s)),
            Some('\\') => match chars.next() {
                Some(c) => s.push(c),
                None => return Err(anyhow!("Unexpected end of string")),
            },
            Some(c) => s.push(c),
            None => return Err(anyhow!("Unterminated string")),
        }
    }
}

fn parse_number<I>(chars: &mut std::iter::Peekable<I>) -> Result<SExpr>
where
    I: Iterator<Item = char>,
{
    let mut num = String::new();

    while let Some(c) = chars.peek() {
        if c.is_digit(10) || *c == '.' || *c == '-' {
            num.push(*c);
            chars.next();
        } else {
            break;
        }
    }

    num.parse::<f64>()
        .map(SExpr::Number)
        .map_err(|e| anyhow!("Invalid number: {e}"))
}

fn parse_symbol<I>(chars: &mut std::iter::Peekable<I>) -> Result<SExpr>
where
    I: Iterator<Item = char>,
{
    let mut sym = String::new();

    while let Some(c) = chars.peek() {
        if c.is_alphanumeric() || is_symbol_char(*c) {
            sym.push(*c);
            chars.next();
        } else {
            break;
        }
    }

    match sym.as_str() {
        "true" => Ok(SExpr::Bool(true)),
        "false" => Ok(SExpr::Bool(false)),
        _ => Ok(SExpr::Symbol(sym)),
    }
}

fn is_symbol_char(c: char) -> bool {
    matches!(
        c,
        '+' | '-' | '*' | '/' | '_' | '?' | '!' | '=' | '<' | '>' | '.' | ':'
    )
}

pub(super) fn eval(env: &Env, expr: &SExpr) -> Result<SExpr> {
    println!("[EVALUATING] {expr:?}");
    match expr {
        // Variables:
        SExpr::Symbol(sym) => {
            if let Some(val) = env.get(sym) {
                Ok(val)
            } else {
                Err(anyhow!("Undefined variable: {sym}"))
            }
        }
        // Literals:
        SExpr::Nil
        | SExpr::Bool(_)
        | SExpr::Number(_)
        | SExpr::String(_)
        | SExpr::Regex(_)
        | SExpr::Event(_)
        | SExpr::Lambda(_, _)
        | SExpr::DateTimeUTC(_)
        | SExpr::NativeFunction(_, _)
        | SExpr::NativeForm(_, _) => Ok(expr.clone()),

        // Function call or lambda expression:
        SExpr::List(elements) => {
            if elements.is_empty() {
                return Ok(SExpr::Nil);
            }

            let mut form = elements.first().cloned();

            while form
                .as_ref()
                .map(|f| match f {
                    SExpr::List(_) | SExpr::Symbol(_) => true,
                    _ => false,
                })
                .unwrap_or(false)
            {
                form = Some(eval(env, &form.unwrap())?);
            }

            match form {
                Some(SExpr::NativeFunction(name, f)) => {
                    // Evaluate all arguments before passing them to the native function
                    let mut evaluated_args = Vec::new();
                    for arg in &elements[1..] {
                        let eval_arg = eval(env, arg)?;
                        evaluated_args.push(eval_arg);
                    }
                    println!("Trying to run: ({name} {evaluated_args:?})");
                    f(&name, evaluated_args)
                }
                Some(SExpr::NativeForm(name, f)) => f(&name, elements[1..].to_vec(), env),
                Some(SExpr::Lambda(params, body)) => {
                    // Evaluate all arguments before passing them to the lambda
                    let mut evaluated_args = Vec::new();
                    for arg in &elements[1..] {
                        let eval_arg = eval(env, arg)?;
                        evaluated_args.push(eval_arg);
                    }

                    // Create a new environment with the parameters bound to the evaluated arguments
                    let f_env = env.with_vars(params.clone(), evaluated_args);
                    eval(&f_env, &body)
                }
                None => Ok(SExpr::Nil),
                e => Err(anyhow!("Cannot call a non-function: {e:?}")),
            }
        }
    }
}

fn native_lambda(_name: &str, elements: Vec<SExpr>, _env: &Env) -> Result<SExpr> {
    if elements.len() >= 2 {
        if let SExpr::List(params_list) = &elements[0] {
            // Extract parameter names
            let mut param_names = Vec::new();
            for param in params_list {
                if let SExpr::Symbol(name) = param {
                    param_names.push(name.clone());
                } else {
                    return Err(anyhow!("Lambda parameters must be symbols"));
                }
            }

            // The body is the third element
            let body = Box::new(elements[1].clone());

            return Ok(SExpr::Lambda(param_names, body));
        } else {
            return Err(anyhow!("Lambda parameters must be a list"));
        }
    }
    Err(anyhow!("Lambda must be called with at least 2 elements"))
}

fn native_funcall(_name: &str, elements: Vec<SExpr>, env: &Env) -> Result<SExpr> {
    if elements.len() < 1 {
        return Err(anyhow!("funcall requires at least 1 argument"));
    }

    let (func_name, val) = match &elements[0] {
        SExpr::Symbol(name) => (name.as_str(), env.get(&name)),
        SExpr::Lambda(_, _) => ("lambda", Some(elements[0].clone())),
        _ => {
            return Err(anyhow!(
                "funcall requires a symbol or lambda as the first argument"
            ))
        }
    };

    if let Some(val) = val {
        match val {
            SExpr::NativeFunction(_, f) => {
                let mut evaluated_args = Vec::new();
                for arg in &elements[1..] {
                    let eval_arg = eval(env, arg)?;
                    evaluated_args.push(eval_arg);
                }
                f(&func_name, evaluated_args)
            }
            SExpr::Lambda(params, body) => {
                let mut evaluated_args = Vec::new();
                for arg in &elements[1..] {
                    let eval_arg = eval(env, arg)?;
                    evaluated_args.push(eval_arg);
                }
                let f_env = env.with_vars(params.clone(), evaluated_args);
                eval(&f_env, &body)
            }
            SExpr::NativeForm(form_name, f) => {
                // For native forms, pass the unevaluated arguments
                f(&form_name, elements[1..].to_vec(), env)
            }
            _ => Err(anyhow!("Cannot call a non-function: {val:?}")),
        }
    } else {
        Err(anyhow!("Undefined variable: {func_name}"))
    }
}

fn native_quote(_name: &str, elements: Vec<SExpr>, _env: &Env) -> Result<SExpr> {
    if elements.len() != 1 {
        return Err(anyhow!("quote requires exactly 1 argument"));
    }
    Ok(elements[0].clone())
}

// Native function implementations
fn native_comparison(name: &str, args: Vec<SExpr>) -> Result<SExpr> {
    match name {
        "not" => match &args[0] {
            SExpr::Bool(b) => return Ok(SExpr::Bool(!b)),
            _ => return Err(anyhow!("not requires a boolean argument")),
        },
        _ => (),
    };

    if args.len() != 2 {
        return Err(anyhow!("{name} requires exactly 2 arguments"));
    }

    let nn = |a| a != 0.;

    let operation: (
        Box<dyn Fn(bool, bool) -> bool>,
        Box<dyn Fn(f64, f64) -> bool>,
    ) = match name {
        "=" => (Box::new(|a, b| a == b), Box::new(|a, b| a == b)),
        "!=" => (Box::new(|a, b| a != b), Box::new(|a, b| a != b)),
        "<" => (Box::new(|a, b| a < b), Box::new(|a, b| a < b)),
        ">" => (Box::new(|a, b| a > b), Box::new(|a, b| a > b)),
        "<=" => (Box::new(|a, b| a <= b), Box::new(|a, b| a <= b)),
        ">=" => (Box::new(|a, b| a >= b), Box::new(|a, b| a >= b)),
        "and" => (Box::new(|a, b| a && b), Box::new(|a, b| nn(a) && nn(b))),
        "or" => (Box::new(|a, b| a || b), Box::new(|a, b| nn(a) || nn(b))),
        _ => return Err(anyhow!("Unsupported comparison operation")),
    };

    match (&args[0], &args[1]) {
        (SExpr::Number(a), SExpr::Number(b)) => Ok(SExpr::Bool(operation.1(*a, *b))),
        (SExpr::Bool(a), SExpr::Bool(b)) => Ok(SExpr::Bool(operation.0(*a, *b))),
        _ => Err(anyhow!("{name} requires boolean or numeric arguments")),
    }
}

fn native_arithmetic(name: &str, args: Vec<SExpr>) -> Result<SExpr> {
    if args.len() != 2 {
        return Err(anyhow!("{name} requires exactly 2 arguments"));
    }

    let operation: Box<dyn Fn(f64, f64) -> f64> = match name {
        "+" => Box::new(|a, b| a + b),
        "-" => Box::new(|a, b| a - b),
        "*" => Box::new(|a, b| a * b),
        "/" => Box::new(|a, b| {
            if b == 0.0 {
                f64::NAN // Handle division by zero
            } else {
                a / b
            }
        }),
        _ => return Err(anyhow!("Unsupported arithmetic operation")),
    };

    match (&args[0], &args[1]) {
        (SExpr::Number(a), SExpr::Number(b)) => {
            let result = operation(*a, *b);
            if result.is_nan() && name == "/" {
                return Err(anyhow!("Division by zero"));
            }
            Ok(SExpr::Number(result))
        }
        _ => Err(anyhow!("{name} requires numeric arguments")),
    }
}

fn native_contains(_name: &str, args: Vec<SExpr>) -> Result<SExpr> {
    if args.len() != 2 {
        return Err(anyhow!("contains requires exactly 2 arguments"));
    }
    match (&args[0], &args[1]) {
        (SExpr::String(substr), SExpr::String(s)) => Ok(SExpr::Bool(s.contains(substr))),
        _ => Err(anyhow!("contains requires string arguments")),
    }
}

fn native_event_field(name: &str, args: Vec<SExpr>) -> Result<SExpr> {
    let getter: Box<dyn Fn(&EventVersion) -> SExpr> = match name {
        "summary" => Box::new(|event| SExpr::from_option_str(event.summary.clone())),
        "description" => Box::new(|event| SExpr::from_option_str(event.description.clone())),
        "location" => Box::new(|event| SExpr::from_option_str(event.location.clone())),
        "dtstart" => Box::new(|event| SExpr::from_option_datetime(event.dtstart)),
        "dtend" => Box::new(|event| SExpr::from_option_datetime(event.dtend)),
        "uid" => Box::new(|event| SExpr::from_option_str(Some(event.id.to_string()))),
        _ => return Err(anyhow!("Unsupported event field")),
    };

    match &args[..] {
        [SExpr::Event(event)] => Ok(getter(event)),
        _ => {
            return Err(anyhow!(
                "{} requires exactly 1 argument, but got {:?} (len={})",
                name,
                args,
                args.len()
            ))
        }
    }
}

fn native_byevent_runner(_name: &str, args: Vec<SExpr>) -> Result<SExpr> {
    if args.len() != 1 {
        return Err(anyhow!("by-event requires exactly 1 argument"));
    }
    match &args[0] {
        SExpr::Lambda(_, _) => {
            // We return a lambda that accepts a list of events and calls native_byevent_runner_internal with it
            Ok(SExpr::Lambda(
                vec!["event-list".to_string()],
                Box::new(SExpr::List(vec![
                    SExpr::NativeForm(
                        "#by-event-internal".to_string(),
                        native_byevent_runner_internal,
                    ),
                    args[0].clone(),
                    SExpr::Symbol("event-list".to_string()),
                ])),
            ))
        }
        _ => Err(anyhow!("by-event requires a function")),
    }
}

/// Internal function, this is called by the former to evaluate something like
/// `(by-event (lambda (event) ...))` into
/// `(lambda (event-list) (#by-event-internal (lambda (event) ...) event-list))`
/// Which in turn should just eval `(lambda (event) ...)` for each element in event-list when called
fn native_byevent_runner_internal(_name: &str, args: Vec<SExpr>, env: &Env) -> Result<SExpr> {
    if args.len() != 2 {
        return Err(anyhow!("by-event requires exactly 2 arguments"));
    }
    match (&args[0], &args[1]) {
        (f @ SExpr::Lambda(_, _), event_list) => {
            let events = match event_list {
                SExpr::List(events) => events,
                SExpr::Symbol(sym) => &env
                    .get(sym)
                    .ok_or(anyhow!("symbol not found"))?
                    .into_list()
                    .ok_or(anyhow!("symbol not found"))?,
                _ => return Err(anyhow!("by-event requires a list of events")),
            };
            Ok(SExpr::List(
                events
                    .into_iter()
                    .flat_map(|event| {
                        let result =
                            native_funcall("#byevent#funcall", vec![f.clone(), event.clone()], env)
                                .unwrap();
                        if result.is_true() {
                            vec![event.clone()]
                        } else {
                            vec![]
                        }
                    })
                    .collect(),
            ))
        }
        _ => Err(anyhow!("by-event-runner requires a function and an event")),
    }
}

fn native_let(_name: &str, elements: Vec<SExpr>, env: &Env) -> Result<SExpr> {
    if elements.len() != 2 {
        return Err(anyhow!(
            "let requires exactly 2 arguments: (let ((var1 value1) (var2 value2)) body)"
        ));
    }

    // Extract bindings list and body
    let bindings = match &elements[0] {
        SExpr::List(bindings) => bindings,
        _ => return Err(anyhow!("First argument to let must be a list of bindings")),
    };

    // Create a new environment with the bindings
    let mut new_env = env.clone();

    // Process each binding pair
    for binding in bindings {
        match binding {
            SExpr::List(pair) if pair.len() == 2 => {
                // Extract variable name
                let var_name = match &pair[0] {
                    SExpr::Symbol(name) => name.clone(),
                    _ => return Err(anyhow!("Binding variable must be a symbol")),
                };

                // Evaluate the value in the current environment
                let value = eval(&new_env, &pair[1])?;

                // Add the binding to the new environment
                new_env.set(var_name, value);
            }
            _ => {
                return Err(anyhow!(
                    "Each binding must be a list of form (variable value)"
                ))
            }
        }
    }

    // Evaluate the body in the new environment
    eval(&new_env, &elements[1])
}

fn native_eval(_name: &str, elements: Vec<SExpr>, env: &Env) -> Result<SExpr> {
    if elements.len() != 1 {
        return Err(anyhow!("eval requires exactly 1 argument"));
    }
    eval(env, &elements[0])
}

#[cfg(test)]
fn filter_events(
    events: impl Iterator<Item = EventVersion>,
    filter: &SExpr,
) -> Result<Vec<EventVersion>> {
    let mut filtered = Vec::new();

    let env = Env::new();

    // First evaluate the filter expression
    let filter_fn = eval(&env, filter)?;
    println!("Filter expr: {:?}", filter_fn);

    let events_as_sexpr = SExpr::List(vec![
        SExpr::NativeForm("quote".to_string(), native_quote),
        SExpr::List(events.map(|e| SExpr::Event(e)).collect()),
    ]);

    // Then apply it to the event
    let result = match filter_fn {
        SExpr::Lambda(_, _) => {
            // Call the lambda with the event
            native_funcall("funcall", vec![filter_fn, events_as_sexpr], &env)?
        }
        _ => {
            // Try to evaluate the filter with the event in scope
            let new_env = env.with_vars(vec!["event".to_string()], vec![events_as_sexpr]);
            eval(&new_env, filter)?
        }
    };

    match result {
        SExpr::List(events) => {
            for event in events {
                match event {
                    SExpr::Event(event) => filtered.push(event),
                    _ => return Err(anyhow!("Expected event, got {:?}", event)),
                }
            }
        }
        _ => return Err(anyhow!("Filter returned unexpected value: {:?}", result)),
    }

    Ok(filtered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::model::ParsedEvent;

    #[test]
    fn test_parse_number() {
        assert!(matches!(parse("42").unwrap(), SExpr::Number(42.0)));
        assert!(matches!(parse("-3.14").unwrap(), SExpr::Number(-3.14)));
    }

    #[test]
    fn test_parse_string() {
        assert!(matches!(
            parse("\"hello\"").unwrap(),
            SExpr::String(s) if s == "hello"
        ));
    }

    #[test]
    fn test_parse_list() {
        match parse("(+ 1 2)").unwrap() {
            SExpr::List(list) => {
                assert_eq!(list.len(), 3);
                assert!(matches!(&list[0], SExpr::Symbol(s) if s == "+"));
                assert!(matches!(&list[1], SExpr::Number(1.0)));
                assert!(matches!(&list[2], SExpr::Number(2.0)));
            }
            _ => panic!("Expected List"),
        }
    }

    #[test]
    fn test_lambda_parsing() {
        use super::*;

        // Create a new environment
        let env = Env::new();

        // Test direct lambda creation
        let lambda_expr = parse("(lambda (x) (+ x 1))").unwrap();
        if let SExpr::List(_) = lambda_expr {
            // Should be converted to Lambda during evaluation
            let evaluated = eval(&env, &lambda_expr).unwrap();
            match evaluated {
                SExpr::Lambda(params, _) => {
                    assert_eq!(params.len(), 1);
                    assert_eq!(params[0], "x");
                }
                _ => panic!("Expected Lambda, got {:?}", evaluated),
            }
        } else {
            panic!("Expected List, got {:?}", lambda_expr);
        }

        // Test lambda application
        let apply_expr = parse("((lambda (x) (+ x 1)) 5)").unwrap();
        let result = eval(&env, &apply_expr).unwrap();
        match result {
            SExpr::Number(n) => assert_eq!(n, 6.0),
            _ => panic!("Expected Number, got {:?}", result),
        }
    }

    #[test]
    fn test_parse_ics_filter_simple() -> Result<()> {
        let event = EventVersion::from(ParsedEvent::new(
            ical::parser::ical::component::IcalEvent {
                properties: vec![
                    ical::property::Property {
                        name: "DTSTAMP".to_string(),
                        params: None,
                        value: Some("20240101T100000".to_string()),
                    },
                    ical::property::Property {
                        name: "UID".to_string(),
                        params: None,
                        value: Some("1234".to_string()),
                    },
                    ical::property::Property {
                        name: "DTSTART".to_string(),
                        params: None,
                        value: Some("20240101T100000".to_string()),
                    },
                    ical::property::Property {
                        name: "RRULE".to_string(),
                        params: None,
                        value: Some("FREQ=DAILY;COUNT=3".to_string()),
                    },
                    ical::property::Property {
                        name: "SUMMARY".to_string(),
                        params: None,
                        value: Some("Meeting".to_string()),
                    },
                    ical::property::Property {
                        name: "DESCRIPTION".to_string(),
                        params: None,
                        value: Some("This is a meeting".to_string()),
                    },
                    ical::property::Property {
                        name: "LOCATION".to_string(),
                        params: None,
                        value: Some("Room 1".to_string()),
                    },
                ],
                alarms: vec![],
            },
        )?);

        let events = vec![event];

        let filter = parse("(by-event (lambda (event) (and (contains \"Meeting\" (summary event)) (contains \"This is a meeting\" (description event)))))").unwrap();

        assert!(matches!(filter.clone(), SExpr::List(list) if list.len() == 2));

        let filtered = filter_events(events.iter().cloned(), &filter).unwrap();

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].summary.as_deref(), Some("Meeting"));
        assert_eq!(
            filtered[0].description.as_deref(),
            Some("This is a meeting")
        );

        // Try a different filter where the event won't be matched
        let filter = parse("(by-event (lambda (event) (and (contains \"Meeting\" (summary event)) (contains \"This is another meeting\" (description event)))))").unwrap();
        let filtered = filter_events(events.iter().cloned(), &filter).unwrap();
        assert_eq!(filtered.len(), 0);
        Ok(())
    }

    #[test]
    fn test_funcall() {
        let mut env = Env::new();
        env.setup_standard_lib();

        // Test funcall with a native function
        let result = parse_and_eval(&env, "(funcall + 1 2)").unwrap();
        assert_eq!(format!("{:?}", result), "Number(3)");

        // Test funcall with a lambda
        let lambda = parse_and_eval(&env, "(lambda (x y) (+ x y))").unwrap();
        env.set("add".to_string(), lambda);
        let result = parse_and_eval(&env, "(funcall add 3 4)").unwrap();
        assert_eq!(format!("{:?}", result), "Number(7)");

        // Test error handling
        let result = parse_and_eval(&env, "(funcall non_existent_function 1 2)");
        assert!(result.is_err());
    }

    fn parse_and_eval(env: &Env, input: &str) -> Result<SExpr> {
        let expr = parse(input)?;
        eval(env, &expr)
    }
}
