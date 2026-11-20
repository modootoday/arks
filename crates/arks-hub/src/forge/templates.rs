//! Template catalogs served by the forge: `.gitignore` templates and license
//! templates (`repo gitignore|license`, `repo create --gitignore|--license`).
//!
//! The catalog text is arks's own: short ignore lists written for this forge, and
//! license templates that carry the SPDX identifier, a copyright line and a pointer
//! to the canonical license text at spdx.org (the license text itself is not
//! reproduced). REST shape: `/gitignore/templates[/{name}]`, `/licenses[/{key}]`.

use super::{Req, RouteResult, not_found, segments};
use crate::store::HubStore;
use serde_json::{Value, json};

pub(crate) struct Gitignore {
    pub name: &'static str,
    pub body: &'static str,
}

pub(crate) const GITIGNORES: &[Gitignore] = &[
    Gitignore {
        name: "C",
        body: "# C build products\n*.o\n*.obj\n*.a\n*.so\n*.dylib\n*.dll\n*.exe\n*.out\n/build/\n",
    },
    Gitignore {
        name: "CMake",
        body: "# CMake build trees\n/build/\nCMakeCache.txt\nCMakeFiles/\ncmake_install.cmake\nCTestTestfile.cmake\n",
    },
    Gitignore {
        name: "Go",
        body: "# Go binaries and test output\n/bin/\n*.test\n*.out\ncoverage.txt\n/vendor/\n",
    },
    Gitignore {
        name: "Java",
        body: "# Java and JVM build products\n*.class\n*.jar\n*.war\n/target/\n/build/\n.gradle/\n",
    },
    Gitignore {
        name: "Node",
        body: "# Node.js dependencies, caches and logs\nnode_modules/\nnpm-debug.log*\n.npm/\ndist/\ncoverage/\n.env\n",
    },
    Gitignore {
        name: "Python",
        body: "# Python bytecode, environments and build products\n__pycache__/\n*.py[cod]\n.venv/\nvenv/\nbuild/\ndist/\n*.egg-info/\n.pytest_cache/\n",
    },
    Gitignore {
        name: "Rust",
        body: "# Rust build output (keep Cargo.lock for binaries)\n/target/\n**/*.rs.bk\n",
    },
    Gitignore {
        name: "Terraform",
        body: "# Terraform working files and state\n.terraform/\n*.tfstate\n*.tfstate.*\ncrash.log\n",
    },
    Gitignore {
        name: "macOS",
        body: "# macOS Finder metadata\n.DS_Store\n.AppleDouble\n.LSOverride\n._*\n",
    },
    Gitignore {
        name: "Windows",
        body: "# Windows shell metadata\nThumbs.db\nehthumbs.db\nDesktop.ini\n$RECYCLE.BIN/\n",
    },
];

pub(crate) struct License {
    pub key: &'static str,
    pub name: &'static str,
    pub spdx: &'static str,
    pub description: &'static str,
    pub permissions: &'static [&'static str],
    pub conditions: &'static [&'static str],
    pub limitations: &'static [&'static str],
    pub featured: bool,
}

const P_ALL: &[&str] = &[
    "commercial-use",
    "modifications",
    "distribution",
    "private-use",
];
const P_PAT: &[&str] = &[
    "commercial-use",
    "modifications",
    "distribution",
    "patent-use",
    "private-use",
];
const L_LW: &[&str] = &["liability", "warranty"];
const L_TLW: &[&str] = &["trademark-use", "liability", "warranty"];

pub(crate) const LICENSES: &[License] = &[
    License {
        key: "mit",
        name: "MIT License",
        spdx: "MIT",
        featured: true,
        description: "Short permissive license: keep the copyright and license notice; no warranty.",
        permissions: P_ALL,
        conditions: &["include-copyright"],
        limitations: L_LW,
    },
    License {
        key: "apache-2.0",
        name: "Apache License 2.0",
        spdx: "Apache-2.0",
        featured: true,
        description: "Permissive license with an express patent grant and a notice-of-changes condition.",
        permissions: P_PAT,
        conditions: &["include-copyright", "document-changes"],
        limitations: L_TLW,
    },
    License {
        key: "gpl-3.0",
        name: "GNU General Public License v3.0",
        spdx: "GPL-3.0",
        featured: true,
        description: "Strong copyleft: distributed works must ship source under the same license.",
        permissions: P_PAT,
        conditions: &[
            "include-copyright",
            "document-changes",
            "disclose-source",
            "same-license",
        ],
        limitations: L_LW,
    },
    License {
        key: "agpl-3.0",
        name: "GNU Affero General Public License v3.0",
        spdx: "AGPL-3.0",
        featured: false,
        description: "Copyleft that also covers network use: offering the work as a service requires source.",
        permissions: P_PAT,
        conditions: &[
            "include-copyright",
            "document-changes",
            "disclose-source",
            "network-use-disclose",
            "same-license",
        ],
        limitations: L_LW,
    },
    License {
        key: "lgpl-3.0",
        name: "GNU Lesser General Public License v3.0",
        spdx: "LGPL-3.0",
        featured: false,
        description: "Weak copyleft for libraries: changes to the library stay open; linking works may differ.",
        permissions: P_PAT,
        conditions: &[
            "include-copyright",
            "disclose-source",
            "document-changes",
            "same-license--library",
        ],
        limitations: L_LW,
    },
    License {
        key: "mpl-2.0",
        name: "Mozilla Public License 2.0",
        spdx: "MPL-2.0",
        featured: false,
        description: "File-level copyleft: modified covered files stay open; larger works may use other terms.",
        permissions: P_PAT,
        conditions: &["disclose-source", "include-copyright", "same-license--file"],
        limitations: L_TLW,
    },
    License {
        key: "bsd-2-clause",
        name: "BSD 2-Clause \"Simplified\" License",
        spdx: "BSD-2-Clause",
        featured: false,
        description: "Permissive license with two conditions about keeping the notice in source and binaries.",
        permissions: P_ALL,
        conditions: &["include-copyright"],
        limitations: L_LW,
    },
    License {
        key: "bsd-3-clause",
        name: "BSD 3-Clause \"New\" or \"Revised\" License",
        spdx: "BSD-3-Clause",
        featured: false,
        description: "BSD 2-Clause plus a no-endorsement clause for contributor names.",
        permissions: P_ALL,
        conditions: &["include-copyright"],
        limitations: L_LW,
    },
    License {
        key: "isc",
        name: "ISC License",
        spdx: "ISC",
        featured: false,
        description: "Permissive license equivalent in effect to MIT, in fewer words.",
        permissions: P_ALL,
        conditions: &["include-copyright"],
        limitations: L_LW,
    },
    License {
        key: "0bsd",
        name: "BSD Zero Clause License",
        spdx: "0BSD",
        featured: false,
        description: "Public-domain-like permissive license with no notice condition.",
        permissions: P_ALL,
        conditions: &[],
        limitations: L_LW,
    },
    License {
        key: "unlicense",
        name: "The Unlicense",
        spdx: "Unlicense",
        featured: false,
        description: "Dedication to the public domain with a permissive fallback.",
        permissions: P_ALL,
        conditions: &[],
        limitations: L_LW,
    },
    License {
        key: "cc0-1.0",
        name: "Creative Commons Zero v1.0 Universal",
        spdx: "CC0-1.0",
        featured: false,
        description: "Public-domain dedication with a fallback license; grants no patent or trademark rights.",
        permissions: &[
            "commercial-use",
            "modifications",
            "distribution",
            "private-use",
        ],
        conditions: &[],
        limitations: &["liability", "trademark-use", "patent-use", "warranty"],
    },
];

pub(crate) fn gitignore(name: &str) -> Option<&'static Gitignore> {
    GITIGNORES
        .iter()
        .find(|g| g.name.eq_ignore_ascii_case(name))
}

pub(crate) fn license(key: &str) -> Option<&'static License> {
    LICENSES
        .iter()
        .find(|l| l.key.eq_ignore_ascii_case(key) || l.spdx.eq_ignore_ascii_case(key))
}

/// The arks license template body (placeholders `[year]` / `[fullname]` as gh shows).
pub(crate) fn license_body(l: &License) -> String {
    format!(
        "SPDX-License-Identifier: {spdx}\n\nCopyright (c) [year] [fullname]\n\n\
         This work is licensed under the {name} ({spdx}).\n\
         The full license text is published at https://spdx.org/licenses/{spdx}.html\n\
         and must accompany copies of this work where the license requires it.\n",
        spdx = l.spdx,
        name = l.name,
    )
}

/// License body with the placeholders filled for a repository bootstrap.
pub(crate) fn license_text(l: &License, holder: &str) -> String {
    let year = super::iso8601(crate::store::now_secs())[..4].to_string();
    license_body(l)
        .replace("[year]", &year)
        .replace("[fullname]", holder)
}

fn license_json(l: &License, full: bool) -> Value {
    let mut v = json!({
        "key": l.key,
        "name": l.name,
        "spdx_id": l.spdx,
        "url": format!("/licenses/{}", l.key),
        "node_id": format!("L_{}", l.key),
    });
    if full {
        v["description"] = json!(l.description);
        v["implementation"] = json!(
            "Add the LICENSE file to the root of the repository and replace [year] and [fullname]."
        );
        v["permissions"] = json!(l.permissions);
        v["conditions"] = json!(l.conditions);
        v["limitations"] = json!(l.limitations);
        v["body"] = json!(license_body(l));
        v["featured"] = json!(l.featured);
        v["html_url"] = json!(format!("https://spdx.org/licenses/{}.html", l.spdx));
    }
    v
}

pub(crate) fn route(_st: &mut HubStore, req: &Req<'_>) -> RouteResult {
    if req.method != "GET" {
        return Ok(None);
    }
    if let Some(seg) = segments(req.path, "/gitignore/templates") {
        return Ok(Some(match seg.as_slice() {
            [] => (
                200,
                json!(GITIGNORES.iter().map(|g| g.name).collect::<Vec<_>>()),
            ),
            [name] => match gitignore(name) {
                Some(g) => (200, json!({"name": g.name, "source": g.body})),
                None => not_found(),
            },
            _ => not_found(),
        }));
    }
    if let Some(seg) = segments(req.path, "/licenses") {
        return Ok(Some(match seg.as_slice() {
            [] => {
                let featured = req.query_param("featured").as_deref() == Some("true");
                (
                    200,
                    Value::Array(
                        LICENSES
                            .iter()
                            .filter(|l| !featured || l.featured)
                            .map(|l| license_json(l, false))
                            .collect(),
                    ),
                )
            }
            [key] => match license(key) {
                Some(l) => (200, license_json(l, true)),
                None => not_found(),
            },
            _ => not_found(),
        }));
    }
    Ok(None)
}
