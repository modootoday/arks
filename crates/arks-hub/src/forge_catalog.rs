//! Local forge template catalogs: `.gitignore` templates and licenses.
//!
//! Shapes follow the GitHub REST reference:
//! - <https://docs.github.com/en/rest/gitignore/gitignore> (`GET /gitignore/templates[/{name}]`)
//! - <https://docs.github.com/en/rest/licenses/licenses> (`GET /licenses[/{license}]`)
//!
//! The local catalog is a small arks-authored set (not GitHub's catalog).
//! License bodies are the standard MIT / ISC texts with GitHub's
//! `[year]` / `[fullname]` placeholders.

use serde_json::{Value, json};

const GITIGNORE: &[(&str, &str)] = &[
    (
        "Node",
        "# Dependencies\nnode_modules/\n\n# Logs\nnpm-debug.log*\nyarn-debug.log*\nyarn-error.log*\n\n# Build output\ndist/\ncoverage/\n",
    ),
    (
        "Python",
        "# Byte-compiled files\n__pycache__/\n*.py[cod]\n\n# Virtual environments\n.venv/\nvenv/\n\n# Packaging\nbuild/\ndist/\n*.egg-info/\n",
    ),
    (
        "Rust",
        "# Build output\n/target/\n\n# Backup files written by rustfmt\n**/*.rs.bk\n\n# Debug information written by MSVC\n*.pdb\n",
    ),
];

struct LicenseDef {
    key: &'static str,
    name: &'static str,
    spdx_id: &'static str,
    description: &'static str,
    body: &'static str,
}

const LICENSES: &[LicenseDef] = &[
    LicenseDef {
        key: "isc",
        name: "ISC License",
        spdx_id: "ISC",
        description: "A permissive license functionally equivalent to the BSD 2-Clause and MIT licenses.",
        body: "ISC License\n\nCopyright (c) [year] [fullname]\n\n\
Permission to use, copy, modify, and/or distribute this software for any\n\
purpose with or without fee is hereby granted, provided that the above\n\
copyright notice and this permission notice appear in all copies.\n\n\
THE SOFTWARE IS PROVIDED \"AS IS\" AND THE AUTHOR DISCLAIMS ALL WARRANTIES\n\
WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF\n\
MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR\n\
ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES\n\
WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN\n\
ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF\n\
OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.\n",
    },
    LicenseDef {
        key: "mit",
        name: "MIT License",
        spdx_id: "MIT",
        description: "A short and simple permissive license with conditions only requiring preservation of copyright and license notices.",
        body: "MIT License\n\nCopyright (c) [year] [fullname]\n\n\
Permission is hereby granted, free of charge, to any person obtaining a copy\n\
of this software and associated documentation files (the \"Software\"), to deal\n\
in the Software without restriction, including without limitation the rights\n\
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell\n\
copies of the Software, and to permit persons to whom the Software is\n\
furnished to do so, subject to the following conditions:\n\n\
The above copyright notice and this permission notice shall be included in all\n\
copies or substantial portions of the Software.\n\n\
THE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR\n\
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,\n\
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE\n\
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER\n\
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,\n\
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE\n\
SOFTWARE.\n",
    },
];

/// `GET /gitignore/templates` → array of names.
pub fn gitignore_names() -> Value {
    Value::Array(GITIGNORE.iter().map(|(n, _)| json!(n)).collect())
}

/// `GET /gitignore/templates/{name}` (case-sensitive) → `{name, source}`.
pub fn gitignore_template(name: &str) -> Option<Value> {
    GITIGNORE
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(n, src)| json!({"name": n, "source": src}))
}

fn license_simple(l: &LicenseDef) -> Value {
    json!({
        "key": l.key,
        "name": l.name,
        "spdx_id": l.spdx_id,
        "url": format!("/licenses/{}", l.key),
        "node_id": Value::Null,
    })
}

/// `GET /licenses` → array of simple license objects.
pub fn licenses() -> Value {
    Value::Array(LICENSES.iter().map(license_simple).collect())
}

/// `GET /licenses/{license}` → full license object (key match, case-insensitive).
pub fn license(key: &str) -> Option<Value> {
    let l = LICENSES.iter().find(|l| l.key.eq_ignore_ascii_case(key))?;
    let mut v = license_simple(l);
    v["description"] = json!(l.description);
    v["body"] = json!(l.body);
    v["featured"] = json!(true);
    Some(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_lookups() {
        assert!(gitignore_template("Rust").is_some());
        assert!(gitignore_template("rust").is_none());
        assert_eq!(license("MIT").unwrap()["spdx_id"], "MIT");
        assert!(license("nope").is_none());
        assert_eq!(licenses().as_array().unwrap().len(), LICENSES.len());
    }
}
