//! A Go `text/template` subset for gh-symmetric `--template` / `-t` (arks-owned).
//!
//! Supported actions: `{{.}}`, `{{.a.b}}`, `{{$}}`, `{{$v}}` / `{{$v.a}}`, string and
//! number literals, pipelines with `|`, `{{if}}…{{else if}}…{{else}}…{{end}}`,
//! `{{range [$i, $e :=] …}}…{{else}}…{{end}}`, `{{with}}…{{end}}`, comments, and the
//! `{{-` / `-}}` trim markers. Functions: the Go builtins `len index printf print
//! println eq ne lt le gt ge and or not`, and gh's template helpers `join pluck
//! tablerow tablerender truncate color autocolor timefmt timeago hyperlink`
//! (non-TTY forms: no colour, `tablerow` writes tab-separated cells). Anything else
//! is a template error (exit 1), never a silent pass-through.

use serde_json::{Value, json};
use std::collections::HashMap;

#[derive(Debug, Clone)]
enum Node {
    Text(String),
    Action(Pipe),
    If(Vec<(Pipe, Vec<Node>)>, Vec<Node>),
    Range(Option<String>, Option<String>, Pipe, Vec<Node>, Vec<Node>),
    With(Pipe, Vec<Node>, Vec<Node>),
}

type Pipe = Vec<Cmd>;

#[derive(Debug, Clone)]
struct Cmd {
    args: Vec<Arg>,
}

#[derive(Debug, Clone)]
enum Arg {
    Dot(Vec<String>),
    Var(String, Vec<String>),
    Str(String),
    Num(f64),
    Bool(bool),
    Nil,
    Func(String),
    Sub(Pipe),
}

fn split_actions(src: &str) -> Result<Vec<(bool, String)>, String> {
    let mut out: Vec<(bool, String)> = Vec::new();
    let mut rest = src;
    while let Some(i) = rest.find("{{") {
        let mut text = rest[..i].to_string();
        let after = &rest[i + 2..];
        let trim_left = after.starts_with('-') && after[1..].starts_with([' ', '\t', '\n']);
        if trim_left {
            text = text.trim_end().to_string();
        }
        let close = after.find("}}").ok_or("unclosed action")?;
        let mut inner = &after[..close];
        if trim_left {
            inner = &inner[1..];
        }
        let trim_right =
            inner.ends_with('-') && inner[..inner.len() - 1].ends_with([' ', '\t', '\n']);
        if trim_right {
            inner = &inner[..inner.len() - 1];
        }
        if !text.is_empty() {
            out.push((false, text));
        }
        out.push((true, inner.trim().to_string()));
        rest = &after[close + 2..];
        if trim_right {
            rest = rest.trim_start();
        }
    }
    if !rest.is_empty() {
        out.push((false, rest.to_string()));
    }
    Ok(out)
}

fn tokenize(s: &str) -> Result<Vec<String>, String> {
    let chars: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if c == '"' || c == '`' {
            let q = c;
            let start = i;
            i += 1;
            while i < chars.len() && chars[i] != q {
                if q == '"' && chars[i] == '\\' {
                    i += 1;
                }
                i += 1;
            }
            if i >= chars.len() {
                return Err("unterminated string".into());
            }
            i += 1;
            out.push(chars[start..i].iter().collect());
        } else if c == '(' || c == ')' || c == '|' {
            out.push(c.to_string());
            i += 1;
        } else {
            let start = i;
            while i < chars.len() && !chars[i].is_whitespace() && !"()|".contains(chars[i]) {
                i += 1;
            }
            out.push(chars[start..i].iter().collect());
        }
    }
    Ok(out)
}

fn unquote(t: &str) -> Result<String, String> {
    if let Some(raw) = t.strip_prefix('`') {
        return Ok(raw.trim_end_matches('`').to_string());
    }
    let inner = &t[1..t.len() - 1];
    let mut s = String::new();
    let mut it = inner.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.next() {
                Some('n') => s.push('\n'),
                Some('t') => s.push('\t'),
                Some('"') => s.push('"'),
                Some('\\') => s.push('\\'),
                Some(o) => return Err(format!("unsupported escape \\{o}")),
                None => {}
            }
        } else {
            s.push(c);
        }
    }
    Ok(s)
}

fn field_path(s: &str) -> Vec<String> {
    s.split('.')
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect()
}

fn parse_pipe(toks: &[String], pos: &mut usize) -> Result<Pipe, String> {
    let mut pipe = Vec::new();
    let mut cmd = Cmd { args: Vec::new() };
    while *pos < toks.len() {
        let t = toks[*pos].as_str();
        *pos += 1;
        match t {
            ")" => {
                *pos -= 1;
                break;
            }
            "|" => {
                pipe.push(std::mem::replace(&mut cmd, Cmd { args: Vec::new() }));
            }
            "(" => {
                let sub = parse_pipe(toks, pos)?;
                if toks.get(*pos).map(String::as_str) != Some(")") {
                    return Err("unclosed (".into());
                }
                *pos += 1;
                cmd.args.push(Arg::Sub(sub));
            }
            _ if t.starts_with('"') || t.starts_with('`') => cmd.args.push(Arg::Str(unquote(t)?)),
            _ if t.starts_with('.') => cmd.args.push(Arg::Dot(field_path(t))),
            _ if t.starts_with('$') => {
                let (name, path) = t.split_once('.').map_or((t, ""), |(a, b)| (a, b));
                cmd.args.push(Arg::Var(name.to_string(), field_path(path)));
            }
            "true" | "false" => cmd.args.push(Arg::Bool(t == "true")),
            "nil" => cmd.args.push(Arg::Nil),
            _ if t.parse::<f64>().is_ok() => cmd.args.push(Arg::Num(t.parse().unwrap_or(0.0))),
            _ => cmd.args.push(Arg::Func(t.to_string())),
        }
    }
    if !cmd.args.is_empty() {
        pipe.push(cmd);
    }
    if pipe.is_empty() {
        return Err("empty pipeline".into());
    }
    Ok(pipe)
}

fn pipe_of(s: &str) -> Result<Pipe, String> {
    let toks = tokenize(s)?;
    let mut pos = 0;
    let p = parse_pipe(&toks, &mut pos)?;
    if pos != toks.len() {
        return Err(format!("unexpected {:?}", toks[pos]));
    }
    Ok(p)
}

struct Parser {
    items: Vec<(bool, String)>,
    pos: usize,
}

impl Parser {
    /// Parse nodes until one of `stops` (returned) or end of input.
    fn nodes(&mut self, stops: &[&str]) -> Result<(Vec<Node>, Option<String>), String> {
        let mut out = Vec::new();
        while self.pos < self.items.len() {
            let (is_action, s) = self.items[self.pos].clone();
            self.pos += 1;
            if !is_action {
                out.push(Node::Text(s));
                continue;
            }
            if s.starts_with("/*") {
                continue;
            }
            let kw = s.split_whitespace().next().unwrap_or("");
            if stops.contains(&kw) {
                return Ok((out, Some(s)));
            }
            match kw {
                "if" => out.push(self.if_node(&s[2..])?),
                "range" => {
                    let spec = s[5..].trim();
                    let (vars, expr) = match spec.split_once(":=") {
                        Some((v, e)) => (Some(v.trim().to_string()), e.trim().to_string()),
                        None => (None, spec.to_string()),
                    };
                    let (iv, ev) = match vars {
                        None => (None, None),
                        Some(v) => match v.split_once(',') {
                            Some((a, b)) => {
                                (Some(a.trim().to_string()), Some(b.trim().to_string()))
                            }
                            None => (None, Some(v)),
                        },
                    };
                    let p = pipe_of(&expr)?;
                    let (body, stop) = self.nodes(&["else", "end"])?;
                    let els = if stop.as_deref() == Some("else") {
                        self.nodes(&["end"])?.0
                    } else {
                        Vec::new()
                    };
                    out.push(Node::Range(iv, ev, p, body, els));
                }
                "with" => {
                    let p = pipe_of(&s[4..])?;
                    let (body, stop) = self.nodes(&["else", "end"])?;
                    let els = if stop.as_deref() == Some("else") {
                        self.nodes(&["end"])?.0
                    } else {
                        Vec::new()
                    };
                    out.push(Node::With(p, body, els));
                }
                "else" | "end" => return Err(format!("unexpected {{{{{kw}}}}}")),
                "define" | "template" | "block" | "break" | "continue" => {
                    return Err(format!("{kw} actions are not supported by arks templates"));
                }
                _ => out.push(Node::Action(pipe_of(&s)?)),
            }
        }
        if stops.is_empty() {
            Ok((out, None))
        } else {
            Err("unexpected end of template (missing {{end}})".into())
        }
    }

    fn if_node(&mut self, cond: &str) -> Result<Node, String> {
        let mut arms = Vec::new();
        let mut cond = pipe_of(cond)?;
        loop {
            let (body, stop) = self.nodes(&["else", "end"])?;
            arms.push((cond, body));
            let stop = stop.unwrap_or_default();
            if stop == "end" {
                return Ok(Node::If(arms, Vec::new()));
            }
            let rest = stop[4..].trim();
            if let Some(c) = rest.strip_prefix("if ") {
                cond = pipe_of(c)?;
                continue;
            }
            let (els, _) = self.nodes(&["end"])?;
            return Ok(Node::If(arms, els));
        }
    }
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(m) => !m.is_empty(),
    }
}

fn text(v: &Value) -> String {
    match v {
        Value::Null => "<no value>".into(),
        Value::String(s) => s.clone(),
        Value::Array(a) => {
            let parts: Vec<String> = a.iter().map(text).collect();
            format!("[{}]", parts.join(" "))
        }
        Value::Object(m) => {
            let parts: Vec<String> = m.iter().map(|(k, v)| format!("{k}:{}", text(v))).collect();
            format!("map[{}]", parts.join(" "))
        }
        other => other.to_string(),
    }
}

fn walk(mut v: Value, path: &[String]) -> Value {
    for p in path {
        v = match v {
            Value::Object(mut m) => m.remove(p).unwrap_or(Value::Null),
            _ => Value::Null,
        };
    }
    v
}

struct Env<'a> {
    root: &'a Value,
    vars: HashMap<String, Value>,
}

fn eval_arg(a: &Arg, dot: &Value, env: &Env<'_>) -> Result<Value, String> {
    Ok(match a {
        Arg::Dot(p) => walk(dot.clone(), p),
        Arg::Var(name, p) => {
            let base = if name == "$" {
                env.root.clone()
            } else {
                env.vars
                    .get(name)
                    .cloned()
                    .ok_or_else(|| format!("undefined variable {name}"))?
            };
            walk(base, p)
        }
        Arg::Str(s) => Value::String(s.clone()),
        Arg::Num(n) => json!(n),
        Arg::Bool(b) => Value::Bool(*b),
        Arg::Nil => Value::Null,
        Arg::Sub(p) => eval_pipe(p, dot, env)?,
        Arg::Func(f) => call(f, &[], dot)?,
    })
}

fn eval_pipe(p: &Pipe, dot: &Value, env: &Env<'_>) -> Result<Value, String> {
    let mut prev: Option<Value> = None;
    for cmd in p {
        let mut vals = Vec::new();
        let head = &cmd.args[0];
        for a in &cmd.args[1..] {
            vals.push(eval_arg(a, dot, env)?);
        }
        if let Some(pv) = prev.take() {
            vals.push(pv);
        }
        let v = match head {
            Arg::Func(f) => call(f, &vals, dot)?,
            other if vals.is_empty() => eval_arg(other, dot, env)?,
            _ => return Err("can't give arguments to a non-function".into()),
        };
        prev = Some(v);
    }
    Ok(prev.unwrap_or(Value::Null))
}

fn num(v: &Value) -> f64 {
    v.as_f64()
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        .unwrap_or(0.0)
}

fn printf(fmt: &str, args: &[Value]) -> String {
    let mut out = String::new();
    let mut it = fmt.chars().peekable();
    let mut ai = 0;
    while let Some(c) = it.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        let mut width = String::new();
        while let Some(d) = it
            .peek()
            .copied()
            .filter(|d| d.is_ascii_digit() || *d == '-')
        {
            width.push(d);
            it.next();
        }
        let verb = it.next().unwrap_or('%');
        if verb == '%' {
            out.push('%');
            continue;
        }
        let a = args.get(ai).cloned().unwrap_or(Value::Null);
        ai += 1;
        let s = match verb {
            'd' => format!("{}", num(&a) as i64),
            'q' => format!("{:?}", text(&a)),
            _ => text(&a),
        };
        let w: i64 = width.parse().unwrap_or(0);
        if w > 0 {
            out.push_str(&format!("{s:>width$}", width = w as usize));
        } else if w < 0 {
            out.push_str(&format!("{s:<width$}", width = (-w) as usize));
        } else {
            out.push_str(&s);
        }
    }
    out
}

fn call(f: &str, args: &[Value], _dot: &Value) -> Result<Value, String> {
    let arg = |i: usize| args.get(i).cloned().unwrap_or(Value::Null);
    let cmp = |op: &str| -> Value {
        let (a, b) = (arg(0), arg(1));
        let ord = match (&a, &b) {
            (Value::String(x), Value::String(y)) => x.cmp(y),
            _ => num(&a)
                .partial_cmp(&num(&b))
                .unwrap_or(std::cmp::Ordering::Equal),
        };
        Value::Bool(match op {
            "lt" => ord.is_lt(),
            "le" => ord.is_le(),
            "gt" => ord.is_gt(),
            _ => ord.is_ge(),
        })
    };
    Ok(match f {
        "len" => json!(match arg(0) {
            Value::String(s) => s.chars().count(),
            Value::Array(a) => a.len(),
            Value::Object(m) => m.len(),
            _ => return Err("len of an unsupported type".into()),
        }),
        "index" => {
            let mut v = arg(0);
            for k in &args[1..] {
                v = match (v, k) {
                    (Value::Array(a), k) => a.get(num(k) as usize).cloned().unwrap_or(Value::Null),
                    (Value::Object(m), Value::String(k)) => {
                        m.get(k).cloned().unwrap_or(Value::Null)
                    }
                    _ => Value::Null,
                };
            }
            v
        }
        "printf" => Value::String(printf(&text(&arg(0)), &args[1.min(args.len())..])),
        "print" => Value::String(args.iter().map(text).collect::<Vec<_>>().join("")),
        "println" => Value::String(format!(
            "{}\n",
            args.iter().map(text).collect::<Vec<_>>().join(" ")
        )),
        "eq" => Value::Bool(args[1.min(args.len())..].iter().any(|b| {
            let a = arg(0);
            a == *b || (a.is_number() && b.is_number() && num(&a) == num(b))
        })),
        "ne" => Value::Bool(arg(0) != arg(1)),
        "lt" | "le" | "gt" | "ge" => cmp(f),
        "and" => args
            .iter()
            .find(|v| !truthy(v))
            .cloned()
            .unwrap_or_else(|| args.last().cloned().unwrap_or(Value::Null)),
        "or" => args
            .iter()
            .find(|v| truthy(v))
            .cloned()
            .unwrap_or_else(|| args.last().cloned().unwrap_or(Value::Null)),
        "not" => Value::Bool(!truthy(&arg(0))),
        "join" => {
            let parts: Vec<String> = arg(1)
                .as_array()
                .map(|a| a.iter().map(text).collect())
                .unwrap_or_default();
            Value::String(parts.join(&text(&arg(0))))
        }
        "pluck" => {
            let key = text(&arg(0));
            Value::Array(
                arg(1)
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .map(|v| v.get(&key).cloned().unwrap_or(Value::Null))
                            .collect()
                    })
                    .unwrap_or_default(),
            )
        }
        "tablerow" => Value::String(format!(
            "{}\n",
            args.iter().map(text).collect::<Vec<_>>().join("\t")
        )),
        "tablerender" => Value::String(String::new()),
        "truncate" => {
            let n = num(&arg(0)) as usize;
            let s = text(&arg(1));
            Value::String(if s.chars().count() > n {
                let keep: String = s.chars().take(n.saturating_sub(3)).collect();
                format!("{keep}...")
            } else {
                s
            })
        }
        "color" | "autocolor" => Value::String(text(&arg(1))),
        "timefmt" => Value::String(text(&arg(1))),
        "timeago" => Value::String(text(&arg(0))),
        "hyperlink" => Value::String(text(&arg(1))),
        other => return Err(format!("function {other:?} not defined")),
    })
}

fn render(nodes: &[Node], dot: &Value, env: &mut Env<'_>, out: &mut String) -> Result<(), String> {
    for n in nodes {
        match n {
            Node::Text(t) => out.push_str(t),
            Node::Action(p) => {
                let v = eval_pipe(p, dot, env)?;
                out.push_str(&text(&v));
            }
            Node::If(arms, els) => {
                let mut done = false;
                for (c, body) in arms {
                    if truthy(&eval_pipe(c, dot, env)?) {
                        render(body, dot, env, out)?;
                        done = true;
                        break;
                    }
                }
                if !done {
                    render(els, dot, env, out)?;
                }
            }
            Node::With(p, body, els) => {
                let v = eval_pipe(p, dot, env)?;
                if truthy(&v) {
                    render(body, &v, env, out)?;
                } else {
                    render(els, dot, env, out)?;
                }
            }
            Node::Range(iv, ev, p, body, els) => {
                let v = eval_pipe(p, dot, env)?;
                let items: Vec<(Value, Value)> = match v {
                    Value::Array(a) => a
                        .into_iter()
                        .enumerate()
                        .map(|(i, x)| (json!(i), x))
                        .collect(),
                    Value::Object(m) => m.into_iter().map(|(k, x)| (Value::String(k), x)).collect(),
                    Value::Null => Vec::new(),
                    Value::Number(n) => (0..n.as_u64().unwrap_or(0))
                        .map(|i| (json!(i), json!(i)))
                        .collect(),
                    other => return Err(format!("range can't iterate over {}", text(&other))),
                };
                if items.is_empty() {
                    render(els, dot, env, out)?;
                }
                for (k, x) in items {
                    if let Some(i) = iv {
                        env.vars.insert(i.clone(), k.clone());
                    }
                    if let Some(e) = ev {
                        env.vars.insert(e.clone(), x.clone());
                    }
                    render(body, &x, env, out)?;
                }
            }
        }
    }
    Ok(())
}

/// Render `tmpl` with `data` as dot (gh `--template` form).
pub(crate) fn run(tmpl: &str, data: &Value) -> Result<String, String> {
    let items = split_actions(tmpl).map_err(|e| format!("template: {e}"))?;
    let mut p = Parser { items, pos: 0 };
    let (nodes, _) = p.nodes(&[]).map_err(|e| format!("template: {e}"))?;
    let mut env = Env {
        root: data,
        vars: HashMap::new(),
    };
    let mut out = String::new();
    render(&nodes, data, &mut env, &mut out).map_err(|e| format!("template: {e}"))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::run;
    use serde_json::json;

    #[test]
    fn fields_range_if_and_helpers() {
        let v = json!([{"number": 1, "title": "a", "labels": [{"name": "bug"}]},
                       {"number": 2, "title": "b", "labels": []}]);
        assert_eq!(
            run("{{range .}}{{.number}} {{.title}}\n{{end}}", &v).unwrap(),
            "1 a\n2 b\n"
        );
        assert_eq!(
            run("{{range .}}{{tablerow .number .title}}{{end}}", &v).unwrap(),
            "1\ta\n2\tb\n"
        );
        assert_eq!(
            run(r#"{{range .}}{{if .labels}}{{join "," (pluck "name" .labels)}}{{else}}-{{end}};{{end}}"#, &v).unwrap(),
            "bug;-;"
        );
        assert_eq!(
            run(
                r#"{{with index . 0}}{{printf "%d-%s" .number "x"}}{{end}}"#,
                &v
            )
            .unwrap(),
            "1-x"
        );
        assert_eq!(run("{{- len . -}}  ", &v).unwrap(), "2");
        assert_eq!(
            run("{{range $i, $e := .}}{{$i}}={{$e.title}} {{end}}", &v).unwrap(),
            "0=a 1=b "
        );
    }

    #[test]
    fn errors_are_reported() {
        assert!(run("{{range .}}", &json!([])).is_err());
        assert!(run("{{nosuch .}}", &json!(1)).is_err());
        assert!(run("{{template \"x\"}}", &json!(1)).is_err());
    }
}
