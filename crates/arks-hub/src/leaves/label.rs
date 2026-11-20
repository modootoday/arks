//! `arks label …` — gh-symmetric label client against the arks forge.

use super::common::{parse_slug, seg};
use super::{Args, Ctx, EXPORT, Fail, Res, arr, json_out, row, s, usage, web};
use serde_json::{Map, Value, json};
use std::io::Write;

pub(crate) const HELP: &str = "\
arks label — repository labels on the arks forge (gh-symmetric argv; host-tested Partial)

USAGE:
  arks label {list|create|edit|delete|clone} [flags]";

const FIELDS: &[&str] = &[
    "color",
    "createdAt",
    "description",
    "id",
    "isDefault",
    "name",
    "updatedAt",
    "url",
];
const R: &[&str] = &["-R", "--repo"];

fn vals(extra: &[&'static str]) -> Vec<&'static str> {
    let mut v: Vec<&str> = R.to_vec();
    v.extend_from_slice(extra);
    v
}

fn color(c: &str) -> Result<String, Fail> {
    let c = c.trim_start_matches('#').to_ascii_lowercase();
    if c.len() == 6 && c.chars().all(|x| x.is_ascii_hexdigit()) {
        Ok(c)
    } else {
        usage(format!("invalid color {c:?}: expected a 6-digit hex value"))
    }
}

/// gh picks a random colour when none is given; arks derives a stable one from the name.
fn auto_color(name: &str) -> String {
    let h = name.bytes().fold(0x811c_9dc5u32, |a, b| {
        (a ^ u32::from(b)).wrapping_mul(0x0100_0193)
    });
    format!("{:06x}", h & 0x00ff_ffff)
}

pub(crate) fn run(ctx: &Ctx, args: &[String], out: &mut dyn Write) -> Res {
    let sub = args.first().map(String::as_str).unwrap_or("");
    let rest = args.get(1..).unwrap_or(&[]);
    match sub {
        "list" | "ls" => {
            let a = Args::parse(
                rest,
                &[
                    vals(&["-L", "--limit", "--order", "-S", "--search", "--sort"]),
                    EXPORT.to_vec(),
                ]
                .concat(),
                &["-w", "--web"],
            )?;
            let (o, r) = ctx.repo(&a)?;
            if a.has(&["-w", "--web"]) {
                return web(out, &ctx.web_url(&format!("/{o}/{r}/labels")));
            }
            let order = a.get(&["--order"]).unwrap_or("asc");
            let sort = a.get(&["--sort"]).unwrap_or("created");
            if !matches!(order, "asc" | "desc") {
                return usage(format!(
                    "invalid argument {order:?} for \"--order\" flag: valid values are {{asc|desc}}"
                ));
            }
            if !matches!(sort, "created" | "name") {
                return usage(format!(
                    "invalid argument {sort:?} for \"--sort\" flag: valid values are {{created|name}}"
                ));
            }
            let mut items = ctx
                .get(&format!("/repos/{o}/{r}/labels"))?
                .as_array()
                .cloned()
                .unwrap_or_default();
            if let Some(q) = a.get(&["-S", "--search"]).map(str::to_ascii_lowercase) {
                items.retain(|l| {
                    s(l, "name").to_ascii_lowercase().contains(&q)
                        || s(l, "description").to_ascii_lowercase().contains(&q)
                });
            }
            if sort == "name" {
                items.sort_by(|x, y| {
                    s(x, "name")
                        .to_ascii_lowercase()
                        .cmp(&s(y, "name").to_ascii_lowercase())
                });
            } else {
                items.sort_by_key(|l| {
                    (
                        l["created_at"].as_str().unwrap_or("").to_string(),
                        l["id"].as_u64().unwrap_or(0),
                    )
                });
            }
            if order == "desc" {
                items.reverse();
            }
            items.truncate(a.limit(30)?);
            let rows: Vec<Map<String, Value>> = items
                .iter()
                .map(|l| {
                    let mut m = Map::new();
                    m.insert("color".into(), l["color"].clone());
                    m.insert("createdAt".into(), l["created_at"].clone());
                    m.insert("description".into(), l["description"].clone());
                    m.insert("id".into(), l["node_id"].clone());
                    m.insert("isDefault".into(), l["default"].clone());
                    m.insert("name".into(), l["name"].clone());
                    m.insert("updatedAt".into(), l["updated_at"].clone());
                    m.insert(
                        "url".into(),
                        json!(ctx.web_url(&format!("/{o}/{r}/labels/{}", seg(s(l, "name"))))),
                    );
                    m
                })
                .collect();
            if json_out(out, &a, &rows, FIELDS, false)? {
                return Ok(());
            }
            if items.is_empty() {
                eprintln!("no labels found in {o}/{r}");
            }
            for l in &items {
                row(
                    out,
                    &[
                        s(l, "name"),
                        s(l, "description"),
                        &format!("#{}", s(l, "color").to_ascii_uppercase()),
                    ],
                )?;
            }
            Ok(())
        }
        "create" => {
            let a = Args::parse(
                rest,
                &vals(&["-c", "--color", "-d", "--description"]),
                &["-f", "--force"],
            )?;
            let (o, r) = ctx.repo(&a)?;
            let name =
                a.positionals().first().copied().ok_or_else(|| {
                    Fail::Usage("cannot create label: name argument required".into())
                })?;
            let col = match a.get(&["-c", "--color"]) {
                Some(c) => color(c)?,
                None => auto_color(name),
            };
            let body = json!({"name": name, "color": col, "description": a.get(&["-d", "--description"]).unwrap_or("")});
            match ctx.api("POST", &format!("/repos/{o}/{r}/labels"), Some(&body)) {
                Ok(_) => {}
                Err(Fail::Api(m)) if m.contains("422") && a.has(&["-f", "--force"]) => {
                    let mut patch = json!({"color": col});
                    if let Some(d) = a.get(&["-d", "--description"]) {
                        patch["description"] = json!(d);
                    }
                    ctx.api(
                        "PATCH",
                        &format!("/repos/{o}/{r}/labels/{}", seg(name)),
                        Some(&patch),
                    )?;
                }
                Err(Fail::Api(m)) if m.contains("422") => {
                    return Err(Fail::Api(format!(
                        "label with name {name:?} already exists; use `--force` to update its color and description"
                    )));
                }
                Err(e) => return Err(e),
            }
            eprintln!("✓ Label \"{name}\" created in {o}/{r}");
            let _ = out;
            Ok(())
        }
        "edit" => {
            let a = Args::parse(
                rest,
                &vals(&["-c", "--color", "-d", "--description", "-n", "--name"]),
                &[],
            )?;
            let (o, r) = ctx.repo(&a)?;
            let name =
                a.positionals().first().copied().ok_or_else(|| {
                    Fail::Usage("cannot update label: name argument required".into())
                })?;
            let mut body = Map::new();
            if let Some(c) = a.get(&["-c", "--color"]) {
                body.insert("color".into(), json!(color(c)?));
            }
            if let Some(d) = a.get(&["-d", "--description"]) {
                body.insert("description".into(), json!(d));
            }
            if let Some(nn) = a.get(&["-n", "--name"]) {
                body.insert("new_name".into(), json!(nn));
            }
            if body.is_empty() {
                return usage("specify at least one of `--color`, `--description`, or `--name`");
            }
            let v = ctx.api(
                "PATCH",
                &format!("/repos/{o}/{r}/labels/{}", seg(name)),
                Some(&Value::Object(body)),
            )?;
            eprintln!("✓ Label \"{name}\" updated in {o}/{r}");
            writeln!(out, "{}", s(&v, "name"))?;
            Ok(())
        }
        "delete" => {
            let a = Args::parse(rest, R, &["--yes", "-y"])?;
            let (o, r) = ctx.repo(&a)?;
            let name =
                a.positionals().first().copied().ok_or_else(|| {
                    Fail::Usage("cannot delete label: name argument required".into())
                })?;
            ctx.api(
                "DELETE",
                &format!("/repos/{o}/{r}/labels/{}", seg(name)),
                None,
            )?;
            eprintln!("✓ Label \"{name}\" deleted from {o}/{r}");
            let _ = out;
            Ok(())
        }
        "clone" => {
            // `--from` is the legacy arks spelling of gh's positional source repository.
            let a = Args::parse(rest, &vals(&["--from"]), &["-f", "--force"])?;
            let (o, r) = ctx.repo(&a)?;
            let src = a
                .positionals()
                .first()
                .copied()
                .or(a.get(&["--from"]))
                .ok_or_else(|| {
                    Fail::Usage("cannot clone labels: source-repository argument required".into())
                })?;
            let (so, sr) = parse_slug(src).ok_or_else(|| {
                Fail::Usage(format!(
                    "expected the \"[HOST/]OWNER/REPO\" format, got {src:?}"
                ))
            })?;
            let labels = ctx.get(&format!("/repos/{so}/{sr}/labels"))?;
            let existing: Vec<String> = arr(&ctx.get(&format!("/repos/{o}/{r}/labels"))?)
                .iter()
                .map(|l| s(l, "name").to_ascii_lowercase())
                .collect();
            let mut cloned = 0;
            for l in arr(&labels) {
                let name = s(l, "name");
                let body =
                    json!({"name": name, "color": l["color"], "description": l["description"]});
                if existing.contains(&name.to_ascii_lowercase()) {
                    if a.has(&["-f", "--force"]) {
                        ctx.api(
                            "PATCH",
                            &format!("/repos/{o}/{r}/labels/{}", seg(name)),
                            Some(&json!({"color": l["color"], "description": l["description"]})),
                        )?;
                        cloned += 1;
                    }
                    continue;
                }
                ctx.api("POST", &format!("/repos/{o}/{r}/labels"), Some(&body))?;
                cloned += 1;
            }
            eprintln!(
                "✓ Cloned {cloned} label{} from {so}/{sr} to {o}/{r}",
                if cloned == 1 { "" } else { "s" }
            );
            writeln!(out, "cloned={cloned}")?;
            Ok(())
        }
        other => usage(format!(
            "unknown command {other:?} for \"arks label\"\n{HELP}"
        )),
    }
}
