//! arks-owned forge features served by `arks serve` ([[adr:root/20261119230000]] items 6–7).
//!
//! Each submodule owns one feature: its stored model, its store operations and its
//! REST routes. Paths and field names follow the GitHub REST shape as a wire
//! convention only; arks defines the semantics. Never spawns git/gh.

pub(crate) mod autolink;
pub(crate) mod discussion;
pub(crate) mod gist;
pub(crate) mod keys;
pub(crate) mod project;

use crate::store::HubStore;
use serde_json::{Value, json};

/// One parsed forge request (path without query).
pub(crate) struct Req<'a> {
    pub method: &'a str,
    pub path: &'a str,
    pub query: &'a str,
    pub body: &'a [u8],
}

impl Req<'_> {
    pub fn json(&self) -> Value {
        serde_json::from_slice(self.body).unwrap_or(json!({}))
    }

    pub fn query_param(&self, key: &str) -> Option<String> {
        self.query.split('&').find_map(|pair| {
            let (k, v) = pair.split_once('=')?;
            (k == key).then(|| crate::serve::urlencoding_lite_decode(v))
        })
    }
}

/// Status + JSON body.
pub(crate) type Resp = (u16, Value);
/// `Ok(None)` = path not owned by this feature.
pub(crate) type RouteResult = Result<Option<Resp>, String>;

/// Try every arks-owned feature router in turn.
pub(crate) fn route(st: &mut HubStore, req: &Req<'_>) -> RouteResult {
    for router in [
        gist::route,
        keys::route,
        project::route,
        discussion::route,
        autolink::route,
    ] {
        if let Some(resp) = router(st, req)? {
            return Ok(Some(resp));
        }
    }
    Ok(None)
}

pub(crate) fn not_found() -> Resp {
    (404, json!({"message": "Not Found"}))
}

pub(crate) fn invalid(msg: impl Into<String>) -> Resp {
    (422, json!({"message": msg.into()}))
}

pub(crate) fn no_content() -> Resp {
    (204, json!({}))
}

pub(crate) fn method_not_allowed() -> Resp {
    (405, json!({"message": "Method Not Allowed"}))
}

/// Allocate the next id from a counter that may be 0 in older db files.
pub(crate) fn next_id(counter: &mut u64) -> u64 {
    if *counter == 0 {
        *counter = 1;
    }
    let id = *counter;
    *counter += 1;
    id
}

pub(crate) fn repo_exists(st: &HubStore, owner: &str, repo: &str) -> bool {
    st.db
        .repos
        .iter()
        .any(|r| r.owner == owner && r.name == repo)
}

/// Split `path` under `prefix` into non-empty segments.
pub(crate) fn segments<'a>(path: &'a str, prefix: &str) -> Option<Vec<&'a str>> {
    let rest = path.strip_prefix(prefix)?;
    Some(rest.split('/').filter(|s| !s.is_empty()).collect())
}

/// Non-empty trimmed string field.
pub(crate) fn str_field<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

/// RFC 3339 UTC timestamp from epoch seconds (civil-from-days, proleptic Gregorian).
pub(crate) fn iso8601(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3_600,
        (rem / 60) % 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso8601_known_points() {
        assert_eq!(iso8601(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso8601(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(iso8601(1_700_000_000), "2023-11-14T22:13:20Z");
    }
}
