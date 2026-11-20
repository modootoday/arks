//! A small jq-language subset for gh-symmetric `--jq` / `-q` (arks-owned evaluator).
//!
//! Supported: `.`, `.a.b`, `."k"`, `.[n]`, `.[]`, `.[a:b]`, postfix `?`, `|`, `,`, `//`,
//! `and` / `or` / `not`, comparisons, `+ - * / %`, literals, arrays, objects,
//! string interpolation `"\(…)"`, `if … then … elif … else … end`, and the common
//! builtins listed in [`call`]. Anything else is a parse error (exit 1), never a
//! silent pass-through. Output follows gh: string results print raw, other results
//! print as compact JSON, one result per line.

use serde_json::{Map, Number, Value, json};

#[derive(Debug, Clone)]
enum Ast {
    Identity,
    Field(Box<Ast>, String, bool),
    Index(Box<Ast>, Box<Ast>, bool),
    Slice(Box<Ast>, Option<Box<Ast>>, Option<Box<Ast>>),
    Iterate(Box<Ast>, bool),
    Lit(Value),
    Str(Vec<StrPart>),
    Array(Option<Box<Ast>>),
    Object(Vec<(Ast, Option<Ast>)>),
    Pipe(Box<Ast>, Box<Ast>),
    Comma(Box<Ast>, Box<Ast>),
    Alt(Box<Ast>, Box<Ast>),
    Bin(Box<Ast>, String, Box<Ast>),
    And(Box<Ast>, Box<Ast>),
    Or(Box<Ast>, Box<Ast>),
    Neg(Box<Ast>),
    If(Vec<(Ast, Ast)>, Option<Box<Ast>>),
    Call(String, Vec<Ast>),
    RecurseAll,
}

#[derive(Debug, Clone)]
enum StrPart {
    Text(String),
    Interp(Ast),
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Dot,
    DotDot,
    Ident(String),
    FieldName(String),
    Str(Vec<RawPart>),
    Num(f64),
    Punct(&'static str),
    Kw(&'static str),
}

#[derive(Debug, Clone, PartialEq)]
enum RawPart {
    Text(String),
    Interp(String),
}

const KEYWORDS: &[&str] = &[
    "if", "then", "elif", "else", "end", "and", "or", "true", "false", "null",
];

fn lex(src: &str) -> Result<Vec<Tok>, String> {
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '#' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c == '.' {
            if chars.get(i + 1) == Some(&'.') {
                out.push(Tok::DotDot);
                i += 2;
                continue;
            }
            let start = i + 1;
            let mut j = start;
            while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_') {
                j += 1;
            }
            if j > start && !chars[start].is_ascii_digit() {
                out.push(Tok::FieldName(chars[start..j].iter().collect()));
                i = j;
            } else if chars.get(start) == Some(&'"') {
                out.push(Tok::Dot);
                i = start;
            } else {
                out.push(Tok::Dot);
                i += 1;
            }
            continue;
        }
        if c == '"' {
            let (parts, next) = lex_string(&chars, i + 1)?;
            out.push(Tok::Str(parts));
            i = next;
            continue;
        }
        if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            let s: String = chars[start..i].iter().collect();
            out.push(Tok::Num(
                s.parse().map_err(|_| format!("invalid number {s}"))?,
            ));
            continue;
        }
        if c.is_alphabetic() || c == '_' || c == '$' {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let s: String = chars[start..i].iter().collect();
            match KEYWORDS.iter().find(|k| **k == s) {
                Some(k) => out.push(Tok::Kw(k)),
                None => out.push(Tok::Ident(s)),
            }
            continue;
        }
        let two: String = chars[i..chars.len().min(i + 2)].iter().collect();
        let p2 = ["==", "!=", "<=", ">=", "//", "?/"]
            .iter()
            .find(|p| two == **p)
            .copied();
        if let Some(p) = p2.filter(|p| *p != "?/") {
            out.push(Tok::Punct(p));
            i += 2;
            continue;
        }
        let p1 = [
            "|", ",", "(", ")", "[", "]", "{", "}", ":", ";", "<", ">", "+", "-", "*", "/", "%",
            "?",
        ]
        .iter()
        .find(|p| p.starts_with(c))
        .copied()
        .ok_or_else(|| format!("unexpected character {c:?}"))?;
        out.push(Tok::Punct(p1));
        i += 1;
    }
    Ok(out)
}

fn lex_string(chars: &[char], mut i: usize) -> Result<(Vec<RawPart>, usize), String> {
    let mut parts = Vec::new();
    let mut cur = String::new();
    while i < chars.len() {
        match chars[i] {
            '"' => {
                if !cur.is_empty() || parts.is_empty() {
                    parts.push(RawPart::Text(cur));
                }
                return Ok((parts, i + 1));
            }
            '\\' => {
                let e = *chars.get(i + 1).ok_or("unterminated string")?;
                i += 2;
                match e {
                    'n' => cur.push('\n'),
                    't' => cur.push('\t'),
                    'r' => cur.push('\r'),
                    '"' => cur.push('"'),
                    '\\' => cur.push('\\'),
                    '/' => cur.push('/'),
                    '(' => {
                        let mut depth = 1;
                        let start = i;
                        while i < chars.len() {
                            match chars[i] {
                                '(' => depth += 1,
                                ')' => {
                                    depth -= 1;
                                    if depth == 0 {
                                        break;
                                    }
                                }
                                _ => {}
                            }
                            i += 1;
                        }
                        if !cur.is_empty() {
                            parts.push(RawPart::Text(std::mem::take(&mut cur)));
                        }
                        parts.push(RawPart::Interp(chars[start..i].iter().collect()));
                        i += 1;
                    }
                    other => return Err(format!("unsupported escape \\{other}")),
                }
            }
            ch => {
                cur.push(ch);
                i += 1;
            }
        }
    }
    Err("unterminated string".into())
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    fn eat_punct(&mut self, p: &str) -> bool {
        if matches!(self.peek(), Some(Tok::Punct(x)) if *x == p) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn eat_kw(&mut self, k: &str) -> bool {
        if matches!(self.peek(), Some(Tok::Kw(x)) if *x == k) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect_punct(&mut self, p: &str) -> Result<(), String> {
        if self.eat_punct(p) {
            Ok(())
        } else {
            Err(format!("expected `{p}`"))
        }
    }

    fn pipe(&mut self) -> Result<Ast, String> {
        let lhs = self.comma()?;
        if self.eat_punct("|") {
            let rhs = self.pipe()?;
            return Ok(Ast::Pipe(Box::new(lhs), Box::new(rhs)));
        }
        Ok(lhs)
    }

    fn comma(&mut self) -> Result<Ast, String> {
        let mut lhs = self.alt()?;
        while self.eat_punct(",") {
            let rhs = self.alt()?;
            lhs = Ast::Comma(Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn alt(&mut self) -> Result<Ast, String> {
        let lhs = self.or()?;
        if self.eat_punct("//") {
            let rhs = self.alt()?;
            return Ok(Ast::Alt(Box::new(lhs), Box::new(rhs)));
        }
        Ok(lhs)
    }

    fn or(&mut self) -> Result<Ast, String> {
        let mut lhs = self.and()?;
        while self.eat_kw("or") {
            let rhs = self.and()?;
            lhs = Ast::Or(Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn and(&mut self) -> Result<Ast, String> {
        let mut lhs = self.cmp()?;
        while self.eat_kw("and") {
            let rhs = self.cmp()?;
            lhs = Ast::And(Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn cmp(&mut self) -> Result<Ast, String> {
        let lhs = self.add()?;
        for op in ["==", "!=", "<=", ">=", "<", ">"] {
            if self.eat_punct(op) {
                let rhs = self.add()?;
                return Ok(Ast::Bin(Box::new(lhs), op.into(), Box::new(rhs)));
            }
        }
        Ok(lhs)
    }

    fn add(&mut self) -> Result<Ast, String> {
        let mut lhs = self.mul()?;
        loop {
            let op = if self.eat_punct("+") {
                "+"
            } else if self.eat_punct("-") {
                "-"
            } else {
                return Ok(lhs);
            };
            let rhs = self.mul()?;
            lhs = Ast::Bin(Box::new(lhs), op.into(), Box::new(rhs));
        }
    }

    fn mul(&mut self) -> Result<Ast, String> {
        let mut lhs = self.unary()?;
        loop {
            let op = if self.eat_punct("*") {
                "*"
            } else if self.eat_punct("/") {
                "/"
            } else if self.eat_punct("%") {
                "%"
            } else {
                return Ok(lhs);
            };
            let rhs = self.unary()?;
            lhs = Ast::Bin(Box::new(lhs), op.into(), Box::new(rhs));
        }
    }

    fn unary(&mut self) -> Result<Ast, String> {
        if self.eat_punct("-") {
            return Ok(Ast::Neg(Box::new(self.postfix()?)));
        }
        self.postfix()
    }

    fn postfix(&mut self) -> Result<Ast, String> {
        let mut t = self.term()?;
        loop {
            match self.peek().cloned() {
                Some(Tok::FieldName(name)) => {
                    self.pos += 1;
                    let opt = self.eat_punct("?");
                    t = Ast::Field(Box::new(t), name, opt);
                }
                Some(Tok::Dot)
                    if matches!(
                        self.toks.get(self.pos + 1),
                        Some(Tok::Str(_) | Tok::Punct("["))
                    ) =>
                {
                    self.pos += 1;
                    if let Some(Tok::Str(parts)) = self.peek().cloned() {
                        self.pos += 1;
                        let name = plain(&parts)?;
                        let opt = self.eat_punct("?");
                        t = Ast::Field(Box::new(t), name, opt);
                    }
                }
                Some(Tok::Punct("[")) => {
                    self.pos += 1;
                    t = self.bracket(t)?;
                }
                _ => return Ok(t),
            }
        }
    }

    fn bracket(&mut self, base: Ast) -> Result<Ast, String> {
        if self.eat_punct("]") {
            let opt = self.eat_punct("?");
            return Ok(Ast::Iterate(Box::new(base), opt));
        }
        if self.eat_punct(":") {
            let hi = self.pipe()?;
            self.expect_punct("]")?;
            return Ok(Ast::Slice(Box::new(base), None, Some(Box::new(hi))));
        }
        let idx = self.pipe()?;
        if self.eat_punct(":") {
            let hi = if matches!(self.peek(), Some(Tok::Punct("]"))) {
                None
            } else {
                Some(Box::new(self.pipe()?))
            };
            self.expect_punct("]")?;
            return Ok(Ast::Slice(Box::new(base), Some(Box::new(idx)), hi));
        }
        self.expect_punct("]")?;
        let opt = self.eat_punct("?");
        Ok(Ast::Index(Box::new(base), Box::new(idx), opt))
    }

    fn term(&mut self) -> Result<Ast, String> {
        let tok = self.peek().cloned().ok_or("unexpected end of expression")?;
        self.pos += 1;
        match tok {
            Tok::Dot => {
                if let Some(Tok::Str(parts)) = self.peek().cloned() {
                    self.pos += 1;
                    return Ok(Ast::Field(Box::new(Ast::Identity), plain(&parts)?, false));
                }
                if matches!(self.peek(), Some(Tok::Punct("["))) {
                    self.pos += 1;
                    return self.bracket(Ast::Identity);
                }
                Ok(Ast::Identity)
            }
            Tok::DotDot => Ok(Ast::RecurseAll),
            Tok::FieldName(name) => {
                let opt = self.eat_punct("?");
                Ok(Ast::Field(Box::new(Ast::Identity), name, opt))
            }
            Tok::Num(n) => Ok(Ast::Lit(num(n))),
            Tok::Str(parts) => {
                let mut out = Vec::new();
                for p in parts {
                    match p {
                        RawPart::Text(t) => out.push(StrPart::Text(t)),
                        RawPart::Interp(src) => out.push(StrPart::Interp(parse(&src)?)),
                    }
                }
                Ok(Ast::Str(out))
            }
            Tok::Kw("true") => Ok(Ast::Lit(Value::Bool(true))),
            Tok::Kw("false") => Ok(Ast::Lit(Value::Bool(false))),
            Tok::Kw("null") => Ok(Ast::Lit(Value::Null)),
            Tok::Kw("if") => {
                let mut arms = Vec::new();
                let cond = self.pipe()?;
                if !self.eat_kw("then") {
                    return Err("expected `then`".into());
                }
                let body = self.pipe()?;
                arms.push((cond, body));
                let mut els = None;
                loop {
                    if self.eat_kw("elif") {
                        let c = self.pipe()?;
                        if !self.eat_kw("then") {
                            return Err("expected `then`".into());
                        }
                        arms.push((c, self.pipe()?));
                    } else if self.eat_kw("else") {
                        els = Some(Box::new(self.pipe()?));
                    } else if self.eat_kw("end") {
                        break;
                    } else {
                        return Err("expected `end`".into());
                    }
                }
                Ok(Ast::If(arms, els))
            }
            Tok::Punct("(") => {
                let e = self.pipe()?;
                self.expect_punct(")")?;
                Ok(e)
            }
            Tok::Punct("[") => {
                if self.eat_punct("]") {
                    return Ok(Ast::Array(None));
                }
                let e = self.pipe()?;
                self.expect_punct("]")?;
                Ok(Ast::Array(Some(Box::new(e))))
            }
            Tok::Punct("{") => {
                let mut entries = Vec::new();
                if self.eat_punct("}") {
                    return Ok(Ast::Object(entries));
                }
                loop {
                    let key = match self.peek().cloned() {
                        Some(Tok::Ident(k)) => {
                            self.pos += 1;
                            Ast::Lit(Value::String(k))
                        }
                        Some(Tok::Kw(k)) => {
                            self.pos += 1;
                            Ast::Lit(Value::String(k.to_string()))
                        }
                        Some(Tok::Str(parts)) => {
                            self.pos += 1;
                            Ast::Lit(Value::String(plain(&parts)?))
                        }
                        Some(Tok::Punct("(")) => {
                            self.pos += 1;
                            let e = self.pipe()?;
                            self.expect_punct(")")?;
                            e
                        }
                        _ => return Err("expected object key".into()),
                    };
                    let val = if self.eat_punct(":") {
                        Some(self.alt()?)
                    } else {
                        None
                    };
                    entries.push((key, val));
                    if self.eat_punct("}") {
                        break;
                    }
                    self.expect_punct(",")?;
                }
                Ok(Ast::Object(entries))
            }
            Tok::Ident(name) => {
                let mut args = Vec::new();
                if self.eat_punct("(") {
                    loop {
                        args.push(self.pipe()?);
                        if self.eat_punct(")") {
                            break;
                        }
                        self.expect_punct(";")?;
                    }
                }
                Ok(Ast::Call(name, args))
            }
            other => Err(format!("unexpected token {other:?}")),
        }
    }
}

fn plain(parts: &[RawPart]) -> Result<String, String> {
    let mut s = String::new();
    for p in parts {
        match p {
            RawPart::Text(t) => s.push_str(t),
            RawPart::Interp(_) => return Err("interpolation not allowed here".into()),
        }
    }
    Ok(s)
}

fn num(n: f64) -> Value {
    if n.fract() == 0.0 && n.abs() < 9.0e15 {
        json!(n as i64)
    } else {
        Number::from_f64(n)
            .map(Value::Number)
            .unwrap_or(Value::Null)
    }
}

fn parse(src: &str) -> Result<Ast, String> {
    let mut p = Parser {
        toks: lex(src)?,
        pos: 0,
    };
    let ast = p.pipe()?;
    if p.pos != p.toks.len() {
        return Err(format!("unexpected token {:?}", p.toks[p.pos]));
    }
    Ok(ast)
}

fn truthy(v: &Value) -> bool {
    !matches!(v, Value::Null | Value::Bool(false))
}

fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn order(v: &Value) -> u8 {
    match v {
        Value::Null => 0,
        Value::Bool(false) => 1,
        Value::Bool(true) => 2,
        Value::Number(_) => 3,
        Value::String(_) => 4,
        Value::Array(_) => 5,
        Value::Object(_) => 6,
    }
}

fn compare(a: &Value, b: &Value) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x
            .as_f64()
            .partial_cmp(&y.as_f64())
            .unwrap_or(Ordering::Equal),
        (Value::String(x), Value::String(y)) => x.cmp(y),
        (Value::Array(x), Value::Array(y)) => {
            for (p, q) in x.iter().zip(y) {
                let o = compare(p, q);
                if o != Ordering::Equal {
                    return o;
                }
            }
            x.len().cmp(&y.len())
        }
        _ => order(a).cmp(&order(b)),
    }
}

fn to_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn one(ast: &Ast, input: &Value) -> Result<Value, String> {
    eval(ast, input)?
        .into_iter()
        .next()
        .ok_or_else(|| "expression produced no value".into())
}

fn eval(ast: &Ast, input: &Value) -> Result<Vec<Value>, String> {
    Ok(match ast {
        Ast::Identity => vec![input.clone()],
        Ast::RecurseAll => {
            let mut out = Vec::new();
            recurse(input, &mut out);
            out
        }
        Ast::Field(base, name, opt) => {
            let mut out = Vec::new();
            for v in eval(base, input)? {
                match v {
                    Value::Object(m) => out.push(m.get(name).cloned().unwrap_or(Value::Null)),
                    Value::Null => out.push(Value::Null),
                    other if !opt => {
                        return Err(format!(
                            "cannot index {} with \"{name}\"",
                            type_name(&other)
                        ));
                    }
                    _ => {}
                }
            }
            out
        }
        Ast::Index(base, idx, opt) => {
            let mut out = Vec::new();
            for v in eval(base, input)? {
                for i in eval(idx, input)? {
                    match (&v, &i) {
                        (Value::Array(a), Value::Number(n)) => {
                            let n = n.as_f64().unwrap_or(0.0) as i64;
                            let k = if n < 0 { a.len() as i64 + n } else { n };
                            out.push(
                                usize::try_from(k)
                                    .ok()
                                    .and_then(|k| a.get(k).cloned())
                                    .unwrap_or(Value::Null),
                            );
                        }
                        (Value::Object(m), Value::String(k)) => {
                            out.push(m.get(k).cloned().unwrap_or(Value::Null));
                        }
                        (Value::Null, _) => out.push(Value::Null),
                        _ if *opt => {}
                        _ => {
                            return Err(format!(
                                "cannot index {} with {}",
                                type_name(&v),
                                type_name(&i)
                            ));
                        }
                    }
                }
            }
            out
        }
        Ast::Slice(base, lo, hi) => {
            let mut out = Vec::new();
            for v in eval(base, input)? {
                let len = match &v {
                    Value::Array(a) => a.len(),
                    Value::String(s) => s.chars().count(),
                    Value::Null => {
                        out.push(Value::Null);
                        continue;
                    }
                    other => return Err(format!("cannot slice {}", type_name(other))),
                } as i64;
                let bound = |e: &Option<Box<Ast>>, d: i64| -> Result<usize, String> {
                    let n = match e {
                        Some(e) => one(e, input)?.as_f64().unwrap_or(d as f64) as i64,
                        None => d,
                    };
                    let n = if n < 0 { len + n } else { n };
                    Ok(n.clamp(0, len) as usize)
                };
                let (a, b) = (bound(lo, 0)?, bound(hi, len)?);
                let b = b.max(a);
                out.push(match v {
                    Value::Array(arr) => Value::Array(arr[a..b].to_vec()),
                    Value::String(s) => Value::String(s.chars().skip(a).take(b - a).collect()),
                    _ => Value::Null,
                });
            }
            out
        }
        Ast::Iterate(base, opt) => {
            let mut out = Vec::new();
            for v in eval(base, input)? {
                match v {
                    Value::Array(a) => out.extend(a),
                    Value::Object(m) => out.extend(m.into_iter().map(|(_, v)| v)),
                    other if !opt => {
                        return Err(format!("cannot iterate over {}", type_name(&other)));
                    }
                    _ => {}
                }
            }
            out
        }
        Ast::Lit(v) => vec![v.clone()],
        Ast::Str(parts) => {
            let mut s = String::new();
            for p in parts {
                match p {
                    StrPart::Text(t) => s.push_str(t),
                    StrPart::Interp(e) => s.push_str(&to_text(&one(e, input)?)),
                }
            }
            vec![Value::String(s)]
        }
        Ast::Array(None) => vec![Value::Array(Vec::new())],
        Ast::Array(Some(e)) => vec![Value::Array(eval(e, input)?)],
        Ast::Object(entries) => {
            let mut objs = vec![Map::new()];
            for (k, v) in entries {
                let keys = eval(k, input)?;
                let mut next = Vec::new();
                for key in keys {
                    let key = match key {
                        Value::String(s) => s,
                        other => {
                            return Err(format!(
                                "object key must be a string, not {}",
                                type_name(&other)
                            ));
                        }
                    };
                    let vals = match v {
                        Some(e) => eval(e, input)?,
                        None => vec![input.get(&key).cloned().unwrap_or(Value::Null)],
                    };
                    for o in &objs {
                        for val in &vals {
                            let mut o2 = o.clone();
                            o2.insert(key.clone(), val.clone());
                            next.push(o2);
                        }
                    }
                }
                objs = next;
            }
            objs.into_iter().map(Value::Object).collect()
        }
        Ast::Pipe(a, b) => {
            let mut out = Vec::new();
            for v in eval(a, input)? {
                out.extend(eval(b, &v)?);
            }
            out
        }
        Ast::Comma(a, b) => {
            let mut out = eval(a, input)?;
            out.extend(eval(b, input)?);
            out
        }
        Ast::Alt(a, b) => {
            let left: Vec<Value> = eval(a, input)
                .unwrap_or_default()
                .into_iter()
                .filter(truthy)
                .collect();
            if left.is_empty() {
                eval(b, input)?
            } else {
                left
            }
        }
        Ast::And(a, b) => {
            let mut out = Vec::new();
            for x in eval(a, input)? {
                if !truthy(&x) {
                    out.push(Value::Bool(false));
                    continue;
                }
                for y in eval(b, input)? {
                    out.push(Value::Bool(truthy(&y)));
                }
            }
            out
        }
        Ast::Or(a, b) => {
            let mut out = Vec::new();
            for x in eval(a, input)? {
                if truthy(&x) {
                    out.push(Value::Bool(true));
                    continue;
                }
                for y in eval(b, input)? {
                    out.push(Value::Bool(truthy(&y)));
                }
            }
            out
        }
        Ast::Neg(e) => {
            let mut out = Vec::new();
            for v in eval(e, input)? {
                let n = v.as_f64().ok_or("cannot negate a non-number")?;
                out.push(num(-n));
            }
            out
        }
        Ast::Bin(a, op, b) => {
            let mut out = Vec::new();
            for y in eval(b, input)? {
                for x in eval(a, input)? {
                    out.push(binop(&x, op, &y)?);
                }
            }
            out
        }
        Ast::If(arms, els) => {
            for (c, body) in arms {
                let cv = one(c, input)?;
                if truthy(&cv) {
                    return eval(body, input);
                }
            }
            match els {
                Some(e) => eval(e, input)?,
                None => vec![input.clone()],
            }
        }
        Ast::Call(name, args) => call(name, args, input)?,
    })
}

fn recurse(v: &Value, out: &mut Vec<Value>) {
    out.push(v.clone());
    match v {
        Value::Array(a) => a.iter().for_each(|x| recurse(x, out)),
        Value::Object(m) => m.values().for_each(|x| recurse(x, out)),
        _ => {}
    }
}

fn json_eq(x: &Value, y: &Value) -> bool {
    match (x, y) {
        (Value::Number(a), Value::Number(b)) => a.as_f64() == b.as_f64(),
        _ => x == y,
    }
}

fn binop(x: &Value, op: &str, y: &Value) -> Result<Value, String> {
    use std::cmp::Ordering::*;
    Ok(match op {
        "==" => Value::Bool(json_eq(x, y)),
        "!=" => Value::Bool(!json_eq(x, y)),
        "<" => Value::Bool(compare(x, y) == Less),
        ">" => Value::Bool(compare(x, y) == Greater),
        "<=" => Value::Bool(compare(x, y) != Greater),
        ">=" => Value::Bool(compare(x, y) != Less),
        "+" => match (x, y) {
            (Value::Null, v) | (v, Value::Null) => v.clone(),
            (Value::Number(a), Value::Number(b)) => {
                num(a.as_f64().unwrap_or(0.0) + b.as_f64().unwrap_or(0.0))
            }
            (Value::String(a), Value::String(b)) => Value::String(format!("{a}{b}")),
            (Value::Array(a), Value::Array(b)) => {
                Value::Array(a.iter().chain(b).cloned().collect())
            }
            (Value::Object(a), Value::Object(b)) => {
                let mut m = a.clone();
                m.extend(b.clone());
                Value::Object(m)
            }
            _ => return Err(format!("cannot add {} and {}", type_name(x), type_name(y))),
        },
        "-" => match (x, y) {
            (Value::Number(a), Value::Number(b)) => {
                num(a.as_f64().unwrap_or(0.0) - b.as_f64().unwrap_or(0.0))
            }
            (Value::Array(a), Value::Array(b)) => {
                Value::Array(a.iter().filter(|v| !b.contains(v)).cloned().collect())
            }
            _ => {
                return Err(format!(
                    "cannot subtract {} from {}",
                    type_name(y),
                    type_name(x)
                ));
            }
        },
        "*" | "/" | "%" => {
            if let (Value::String(s), Value::String(sep), "/") = (x, y, op) {
                return Ok(Value::Array(
                    s.split(sep.as_str())
                        .map(|p| Value::String(p.into()))
                        .collect(),
                ));
            }
            let (a, b) = (
                x.as_f64().ok_or("arithmetic on a non-number")?,
                y.as_f64().ok_or("arithmetic on a non-number")?,
            );
            match op {
                "*" => num(a * b),
                "/" if b == 0.0 => return Err("division by zero".into()),
                "/" => num(a / b),
                _ if b == 0.0 => return Err("modulo by zero".into()),
                _ => num((a as i64 % b as i64) as f64),
            }
        }
        other => return Err(format!("unsupported operator {other}")),
    })
}

fn arg_one(args: &[Ast], i: usize, input: &Value) -> Result<Value, String> {
    let a = args.get(i).ok_or("missing function argument")?;
    one(a, input)
}

fn str_arg(args: &[Ast], i: usize, input: &Value) -> Result<String, String> {
    match arg_one(args, i, input)? {
        Value::String(s) => Ok(s),
        other => Err(format!(
            "expected a string argument, got {}",
            type_name(&other)
        )),
    }
}

fn call(name: &str, args: &[Ast], input: &Value) -> Result<Vec<Value>, String> {
    let arr = || -> Result<&Vec<Value>, String> {
        input
            .as_array()
            .ok_or_else(|| format!("{name} needs an array, got {}", type_name(input)))
    };
    let s = || -> Result<&str, String> {
        input
            .as_str()
            .ok_or_else(|| format!("{name} needs a string, got {}", type_name(input)))
    };
    Ok(match (name, args.len()) {
        ("empty", 0) => vec![],
        ("not", 0) => vec![Value::Bool(!truthy(input))],
        ("length", 0) => vec![match input {
            Value::Null => json!(0),
            Value::Bool(_) => return Err("boolean has no length".into()),
            Value::Number(n) => num(n.as_f64().unwrap_or(0.0).abs()),
            Value::String(s) => json!(s.chars().count()),
            Value::Array(a) => json!(a.len()),
            Value::Object(m) => json!(m.len()),
        }],
        ("keys", 0) | ("keys_unsorted", 0) => match input {
            Value::Object(m) => {
                let mut k: Vec<String> = m.keys().cloned().collect();
                if name == "keys" {
                    k.sort();
                }
                vec![Value::Array(k.into_iter().map(Value::String).collect())]
            }
            Value::Array(a) => vec![Value::Array((0..a.len()).map(|i| json!(i)).collect())],
            other => return Err(format!("{} has no keys", type_name(other))),
        },
        ("values", 0) => match input {
            Value::Null => vec![],
            v => vec![v.clone()],
        },
        ("type", 0) => vec![json!(type_name(input))],
        ("tostring", 0) => vec![Value::String(to_text(input))],
        ("tonumber", 0) => vec![match input {
            Value::Number(_) => input.clone(),
            Value::String(t) => num(t
                .trim()
                .parse::<f64>()
                .map_err(|_| format!("cannot parse {t:?} as a number"))?),
            other => return Err(format!("cannot convert {} to a number", type_name(other))),
        }],
        ("ascii_downcase", 0) => vec![Value::String(s()?.to_ascii_lowercase())],
        ("ascii_upcase", 0) => vec![Value::String(s()?.to_ascii_uppercase())],
        ("first", 0) => vec![arr()?.first().cloned().unwrap_or(Value::Null)],
        ("last", 0) => vec![arr()?.last().cloned().unwrap_or(Value::Null)],
        ("first", 1) => eval(&args[0], input)?.into_iter().take(1).collect(),
        ("reverse", 0) => vec![match input {
            Value::String(t) => Value::String(t.chars().rev().collect()),
            Value::Null => json!([]),
            _ => Value::Array(arr()?.iter().rev().cloned().collect()),
        }],
        ("sort", 0) => {
            let mut a = arr()?.clone();
            a.sort_by(compare);
            vec![Value::Array(a)]
        }
        ("sort_by", 1) | ("group_by", 1) | ("unique_by", 1) | ("min_by", 1) | ("max_by", 1) => {
            let mut keyed = Vec::new();
            for v in arr()? {
                keyed.push((Value::Array(eval(&args[0], v)?), v.clone()));
            }
            keyed.sort_by(|a, b| compare(&a.0, &b.0));
            match name {
                "sort_by" => vec![Value::Array(keyed.into_iter().map(|(_, v)| v).collect())],
                "min_by" => vec![keyed.first().map(|k| k.1.clone()).unwrap_or(Value::Null)],
                "max_by" => vec![keyed.last().map(|k| k.1.clone()).unwrap_or(Value::Null)],
                _ => {
                    let mut groups: Vec<(Value, Vec<Value>)> = Vec::new();
                    for (k, v) in keyed {
                        match groups.last_mut() {
                            Some((gk, g)) if *gk == k => g.push(v),
                            _ => groups.push((k, vec![v])),
                        }
                    }
                    if name == "group_by" {
                        vec![Value::Array(
                            groups.into_iter().map(|(_, g)| Value::Array(g)).collect(),
                        )]
                    } else {
                        vec![Value::Array(
                            groups
                                .into_iter()
                                .filter_map(|(_, g)| g.into_iter().next())
                                .collect(),
                        )]
                    }
                }
            }
        }
        ("unique", 0) => {
            let mut a = arr()?.clone();
            a.sort_by(compare);
            a.dedup();
            vec![Value::Array(a)]
        }
        ("min", 0) | ("max", 0) => {
            let mut a = arr()?.clone();
            a.sort_by(compare);
            vec![
                if name == "min" { a.first() } else { a.last() }
                    .cloned()
                    .unwrap_or(Value::Null),
            ]
        }
        ("add", 0) => {
            let mut acc = Value::Null;
            for v in arr()? {
                acc = binop(&acc, "+", v)?;
            }
            vec![acc]
        }
        ("any", 0) => vec![Value::Bool(arr()?.iter().any(truthy))],
        ("all", 0) => vec![Value::Bool(arr()?.iter().all(truthy))],
        ("any", 1) | ("all", 1) => {
            let mut hits = Vec::new();
            for v in arr()? {
                hits.push(truthy(&one(&args[0], v)?));
            }
            vec![Value::Bool(if name == "any" {
                hits.iter().any(|h| *h)
            } else {
                hits.iter().all(|h| *h)
            })]
        }
        ("map", 1) => {
            let mut out = Vec::new();
            for v in arr()? {
                out.extend(eval(&args[0], v)?);
            }
            vec![Value::Array(out)]
        }
        ("map_values", 1) => match input {
            Value::Object(m) => {
                let mut o = Map::new();
                for (k, v) in m {
                    if let Some(x) = eval(&args[0], v)?.into_iter().next() {
                        o.insert(k.clone(), x);
                    }
                }
                vec![Value::Object(o)]
            }
            _ => call("map", args, input)?,
        },
        ("select", 1) => {
            let mut out = Vec::new();
            for c in eval(&args[0], input)? {
                if truthy(&c) {
                    out.push(input.clone());
                }
            }
            out
        }
        ("has", 1) => {
            let k = arg_one(args, 0, input)?;
            vec![Value::Bool(match (input, &k) {
                (Value::Object(m), Value::String(k)) => m.contains_key(k),
                (Value::Array(a), Value::Number(n)) => {
                    (n.as_f64().unwrap_or(-1.0) as usize) < a.len()
                }
                _ => {
                    return Err(format!(
                        "cannot check whether {} has a key",
                        type_name(input)
                    ));
                }
            })]
        }
        ("contains", 1) => vec![Value::Bool(contains(input, &arg_one(args, 0, input)?))],
        ("inside", 1) => vec![Value::Bool(contains(&arg_one(args, 0, input)?, input))],
        ("startswith", 1) => vec![Value::Bool(s()?.starts_with(&str_arg(args, 0, input)?))],
        ("endswith", 1) => vec![Value::Bool(s()?.ends_with(&str_arg(args, 0, input)?))],
        ("ltrimstr", 1) => {
            let p = str_arg(args, 0, input)?;
            vec![match input.as_str() {
                Some(t) => Value::String(t.strip_prefix(p.as_str()).unwrap_or(t).into()),
                None => input.clone(),
            }]
        }
        ("rtrimstr", 1) => {
            let p = str_arg(args, 0, input)?;
            vec![match input.as_str() {
                Some(t) => Value::String(t.strip_suffix(p.as_str()).unwrap_or(t).into()),
                None => input.clone(),
            }]
        }
        ("split", 1) => {
            let sep = str_arg(args, 0, input)?;
            vec![Value::Array(
                s()?.split(sep.as_str())
                    .map(|p| Value::String(p.into()))
                    .collect(),
            )]
        }
        ("join", 1) => {
            let sep = str_arg(args, 0, input)?;
            let parts: Vec<String> = arr()?
                .iter()
                .map(|v| match v {
                    Value::Null => String::new(),
                    other => to_text(other),
                })
                .collect();
            vec![Value::String(parts.join(&sep))]
        }
        ("test", 1) => vec![Value::Bool(s()?.contains(&str_arg(args, 0, input)?))],
        ("to_entries", 0) => match input {
            Value::Object(m) => vec![Value::Array(
                m.iter()
                    .map(|(k, v)| json!({"key": k, "value": v}))
                    .collect(),
            )],
            other => return Err(format!("{} has no entries", type_name(other))),
        },
        ("from_entries", 0) => {
            let mut m = Map::new();
            for e in arr()? {
                let k = e
                    .get("key")
                    .or_else(|| e.get("name"))
                    .map(to_text)
                    .ok_or("from_entries needs key fields")?;
                m.insert(k, e.get("value").cloned().unwrap_or(Value::Null));
            }
            vec![Value::Object(m)]
        }
        ("with_entries", 1) => {
            let entries = call("to_entries", &[], input)?;
            let mapped = call("map", args, &entries[0])?;
            call("from_entries", &[], &mapped[0])?
        }
        ("tojson", 0) => vec![Value::String(input.to_string())],
        ("fromjson", 0) => vec![serde_json::from_str(s()?).map_err(|e| e.to_string())?],
        ("range", 1) => {
            let n = arg_one(args, 0, input)?.as_f64().unwrap_or(0.0) as i64;
            (0..n).map(|i| json!(i)).collect()
        }
        ("floor", 0) => vec![num(input.as_f64().ok_or("floor needs a number")?.floor())],
        ("env", 0) => vec![Value::Object(
            std::env::vars()
                .map(|(k, v)| (k, Value::String(v)))
                .collect(),
        )],
        ("ascii", 0) | ("@text", 0) => vec![Value::String(to_text(input))],
        (other, n) => return Err(format!("{other}/{n} is not defined")),
    })
}

fn contains(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::String(x), Value::String(y)) => x.contains(y.as_str()),
        (Value::Array(x), Value::Array(y)) => {
            y.iter().all(|yv| x.iter().any(|xv| contains(xv, yv)))
        }
        (Value::Object(x), Value::Object(y)) => y
            .iter()
            .all(|(k, yv)| x.get(k).is_some_and(|xv| contains(xv, yv))),
        _ => a == b,
    }
}

/// Evaluate `expr` against `input`; one output line per result (gh `--jq` form).
pub(crate) fn run(expr: &str, input: &Value) -> Result<Vec<String>, String> {
    let ast = parse(expr).map_err(|e| format!("failed to parse jq expression {expr:?}: {e}"))?;
    let vals = eval(&ast, input)?;
    Ok(vals
        .iter()
        .map(|v| match v {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::run;
    use serde_json::json;

    fn j(expr: &str, v: serde_json::Value) -> Vec<String> {
        run(expr, &v).unwrap_or_else(|e| panic!("{expr}: {e}"))
    }

    #[test]
    fn paths_iteration_and_strings() {
        let v = json!([{"number": 1, "title": "a", "labels": [{"name": "bug"}]},
                       {"number": 2, "title": "b", "labels": []}]);
        assert_eq!(j(".[].number", v.clone()), ["1", "2"]);
        assert_eq!(j(".[0].title", v.clone()), ["a"]);
        assert_eq!(j("map(.number) | add", v.clone()), ["3"]);
        assert_eq!(j(".[] | select(.number > 1) | .title", v.clone()), ["b"]);
        assert_eq!(
            j(r#".[] | "\(.number): \(.title)""#, v.clone()),
            ["1: a", "2: b"]
        );
        assert_eq!(j("length", v.clone()), ["2"]);
        assert_eq!(j("[.[].labels[].name] | join(\",\")", v.clone()), ["bug"]);
        assert_eq!(
            j(".[0] | {n: .number, title}", v.clone()),
            [r#"{"n":1,"title":"a"}"#]
        );
        assert_eq!(j(".[1].missing // \"none\"", v.clone()), ["none"]);
        assert_eq!(
            j("if .[0].number == 1 then \"yes\" else \"no\" end", v),
            ["yes"]
        );
    }

    #[test]
    fn errors_are_reported() {
        assert!(run(".[", &json!([])).is_err());
        assert!(run("nosuchfn", &json!(1)).is_err());
        assert!(run(".a", &json!(5)).is_err());
    }
}
