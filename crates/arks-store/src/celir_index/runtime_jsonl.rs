//! Optional JSONL immutable-root path via `celir-index-runtime`.
//!
//! Cites `publication::build` / `Snapshot::open` / `query`
//! (discrete-circuit indexing-service pack; do not invent APIs).

use celir_index_runtime::{
    build, Budget, BuildReport, Progress, Projection, Query, QueryLimits, Snapshot, Source,
};
use std::io;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

/// Default projection for arks lab JSONL: `{"oid","kind","effect"}`.
pub fn oid_kind_projection() -> Projection {
    Projection {
        columns: vec!["oid".into(), "kind".into()],
        input_paths: vec![vec!["oid".into()], vec!["kind".into()]],
        effect_path: vec!["effect".into()],
    }
}

/// Build + publish an immutable root from JSONL sources (generation must advance).
pub fn build_jsonl_root(
    root: impl AsRef<Path>,
    generation: u64,
    sources: impl IntoIterator<Item = Source>,
) -> io::Result<BuildReport> {
    build(
        root.as_ref(),
        generation,
        sources,
        oid_kind_projection(),
        Budget::default(),
    )
}

/// Open a published immutable root for query.
pub fn open_jsonl_root(root: impl AsRef<Path>) -> io::Result<Snapshot> {
    Snapshot::open(root.as_ref(), Budget::default(), QueryLimits::default())
}

/// Collect exact-match effects for `(oid, kind)`.
pub fn query_exact_effects(
    snapshot: Snapshot,
    oid: &str,
    kind: &str,
) -> io::Result<Vec<String>> {
    let mut cursor = snapshot.query(Query::Exact(vec![oid.to_owned(), kind.to_owned()]))?;
    let mut effects = Vec::new();
    loop {
        let state = cursor.visit_page(64, |row| {
            effects.push(row.effect.to_owned());
            ControlFlow::Continue(())
        })?;
        if state == Progress::Complete {
            break;
        }
    }
    Ok(effects)
}

/// Write a one-line JSONL source for lab rebuilds.
pub fn write_oid_kind_jsonl(
    path: impl AsRef<Path>,
    oid: &str,
    kind: &str,
    effect: &str,
) -> io::Result<PathBuf> {
    let path = path.as_ref().to_path_buf();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let line = format!(
        "{{\"oid\":{oid:?},\"kind\":{kind:?},\"effect\":{effect:?}}}\n"
    );
    std::fs::write(&path, line)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn runtime_build_open_query_roundtrip() {
        let dir = TempDir::new().unwrap();
        let root = dir.path().join("runtime-root");
        let jsonl = write_oid_kind_jsonl(
            dir.path().join("rows.jsonl"),
            "abc123",
            "blob",
            "4",
        )
        .unwrap();
        let report = build_jsonl_root(
            &root,
            1,
            [Source {
                path: jsonl,
                source_id: 1,
                first_record: 1,
            }],
        )
        .expect("build");
        assert_eq!(report.generation, 1);
        assert_eq!(report.rows, 1);
        let snap = open_jsonl_root(&root).expect("open");
        let effects = query_exact_effects(snap, "abc123", "blob").expect("query");
        assert_eq!(effects, vec!["4".to_owned()]);
    }
}
