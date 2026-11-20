//! Mail helpers shared by `am`: RFC 2822 dates and the `--3way` post-image.

use arks_object::{FilePatch, ObjectKind, apply_hunks};
use arks_repo::Repository;

/// RFC 2822 `[Wed, ]15 Nov 2023 05:13:20 +0000` → (unix seconds, `+hhmm`).
pub(crate) fn parse_rfc2822_date(v: &str) -> Option<(i64, String)> {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let v = v.split_once(", ").map(|(_, r)| r).unwrap_or(v);
    let mut it = v.split_whitespace();
    let day: u32 = it.next()?.parse().ok()?;
    let mon_name = it.next()?;
    let mon = MONTHS.iter().position(|m| *m == mon_name)? + 1;
    let year: i64 = it.next()?.parse().ok()?;
    let time = it.next()?;
    let tz = it.next()?;
    crate::cmd::shared::parse_git_date(&format!("{year:04}-{mon:02}-{day:02} {time} {tz}"))
}

/// Claimed `--3way` subset: when the hunks do not apply to HEAD, take the
/// post-image blob named on the `index` line, or re-apply the hunks to the
/// pre-image blob, when that blob is in the object store.
pub(crate) fn three_way_tip(repo: &Repository, fp: &FilePatch) -> Option<Vec<u8>> {
    let (pre, post) = fp.index_oids.as_ref()?;
    let blob = |hex: &str| {
        crate::cmd::shared::resolve_rev(repo, hex)
            .ok()
            .and_then(|o| repo.get_object(&o).ok())
            .filter(|o| o.kind == ObjectKind::Blob)
            .map(|o| o.payload)
    };
    blob(post).or_else(|| apply_hunks(&blob(pre)?, fp).ok())
}
