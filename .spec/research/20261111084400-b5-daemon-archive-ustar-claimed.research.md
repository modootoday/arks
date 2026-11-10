---
id: research:root/20261111084400
title: B5 daemon upload-archive ustar Claimed — header encoding path
status: active
created: 2026-11-10T23:44:00Z
updated: 2026-11-10T23:44:00Z
tags: [b5, claimed, residual, daemon, upload-archive, ustar, git-daemon, clean-room, no-invent-claimed, docker]
verify: manual
rel:
  depends_on:
    - sot:root/arks-claimed-residual-gates
    - sot:root/arks-daemon-product-intent
    - sot:root/arks-daemon-git-command-matrix
    - sot:root/verify-directory-conventions
    - sot:root/product-verification-clean-room
    - research:root/20261111074523
    - evidence:root/20261110232923
  related:
    - verify/scripts/run-claimed-daemon-archive-e2e.sh
    - crates/arks-server/src/daemon.rs
    - verify/images.lock.json
---

# B5 daemon upload-archive ustar Claimed — header encoding path

English-only. Tip surveyed after verify Migration `g11-l3` / `g19-eval.py` (canon ≥ `9b27ea5`). **Design ≠ Claimed.** No invent Claimed from Partial structural greens or opportunistic `cmp`.

## Purpose

Decide how to implement **byte-identical** `upload-archive` ustar vs stock **git-daemon** pin **2.39.5** so B5 can graduate from **Partial (terminal for prior goal)** without inventing Claimed. Known Docker class **501009** already freezes the mismatch: header encoding, not member list / payload.

## Root cause vs git-daemon / `git archive`

### What already matches (Partial — keep)

Clean-room residual evidence ([[evidence:root/20261110232923]] and superseding honesty notes) shows agreement on:

- Protocol path: `git-upload-archive` / `git archive --remote=git://…`
- Structural members (names), types, modes (664/775), payload bytes
- High-level packing shape: `pax_global_header` + tree order + 20×512 block padding (arks `write_ustar_archive` documents stock 2.39.5 intent)

SoT already treats this as **Partial**, not Claimed ([[sot:root/arks-claimed-residual-gates]] B5).

### What blocks Claimed (501009)

| Observation | Meaning |
|-------------|---------|
| First differ @ block0 offset **151** | Inside ustar **chksum** field (bytes 148–155); stock digit `0x34` vs arks `0x33` on diagnosed run |
| Follow-on differ ~offsets **329+** | **devmajor / devminor** (8+8): stock **ASCII `0`** digits vs arks **NUL** bytes on diagnosed run |
| Semantic `tarfile` equality | Python/tarfile agrees → mismatch is **raw header encoding**, not archive membership |

Checksum digits are a **symptom**: ustar chksum is the unsigned byte sum of the 512-byte header with the chksum field itself treated as eight spaces (`ustar_header_chksum` in git `archive-tar.c`). Any NUL-vs-`0` drift in earlier/later numeric fields changes the sum and therefore the printed chksum at offset 151.

### Stock encoder (normative oracle)

Pin source: git **v2.39.5** `archive-tar.c` → `prepare_header`:

1. Numeric fields via `xsnprintf(field, sizeof(field), …)`:
   - mode / uid / gid / **devmajor** / **devminor** / **chksum**: `"%07o"` into **8-byte** slots → **7 octal digits + NUL**
   - size / mtime: `"%011o"` into **12-byte** slots → **11 octal digits + NUL**
2. Always writes device major/minor as `"%07o"` of `0` (not left all-NUL).
3. Magic: six bytes `ustar\0`; version `00`; uname/gname `root`.
4. Chksum computed with chksum bytes as spaces, then written with the same `"%07o"` rule (7 digits + NUL — **not** the older GNU “6 digits + NUL + space” layout).

`git daemon --enable=upload-archive` serves the same tar archiver path clients hit with `git archive --remote`. Claimed compare **MUST** use clean-room stock Git **2.39.5** as both daemon and client oracle ([[sot:root/product-verification-clean-room]]).

### Arks encoder (current intent)

`crates/arks-server/src/daemon.rs` `write_ustar_entry` / `write_octal` already aims at zero-filled octal + trailing NUL and explicitly writes device fields. Residual **501009** still forbids Claimed until a pin Docker receipt proves `cmp -s` identity. Do **not** treat WIP host hex dumps as graduation.

Likely failure modes to close in the implement wave (fail-closed checklist):

1. Any numeric field still left **all-NUL** instead of `"%07o"`/`"%011o"`-shaped bytes.
2. Chksum written with a non-git width (e.g. 6-digit + space) or summed without space-substitution.
3. Mode/mtime/pax comment / umask-equivalent drift that changes header bytes while `tarfile` still “agrees”.
4. Comparing against host Git ≠ 2.39.5 (advisory skew only).

## Options

| Option | Summary | Claimed fitness |
|--------|---------|-----------------|
| **A. Match git `prepare_header` exactly (recommended)** | Keep hand-rolled writer; make every field byte-identical to v2.39.5 `archive-tar.c`; golden vectors from clean-room `git archive` | **Yes** — only path that targets the SoT bar |
| **B. Structural-only Claimed redefine** | Graduate on tarfile semantic equality | **Refuse** — SoT strict bar is byte-identical; already Partial |
| **C. Adopt Rust `tar` / `tokio-tar` / libarchive bindings** | Replace writer with a crate | **Poor default** — libarchive-style fields often use **space-NUL** numeric termination; diverges from git `"%07o"`; adds deps without guaranteeing pin identity |
| **D. Shell out to `git archive` inside product** | Product spawns git | **Forbidden** — product MUST NOT spawn git |
| **E. Stop at Partial forever** | Document 501009 as permanent | Allowed as product decision, but **not** an implement path for Claimed |

### Libraries (honesty)

- Prefer **no new tar crate** for Claimed: the oracle is git’s encoder, not POSIX-ish “any ustar”.
- If a crate is evaluated, it needs a **byte-level** golden test against pin `git archive` before replacing `write_ustar_*`; expect rejection.
- Test-only helpers (Python `tarfile`, `cmp`, small hex classifiers) stay in `verify/scripts/` — never sole Claimed judge without `cmp -s` + SoT amend.

## Recommended implementation path

1. **Spec stays owner of the bar** — keep B5 Claimed = byte-identical ustar vs stock daemon pin 2.39.5; Partial = protocol + structural/semantic ([[sot:root/arks-claimed-residual-gates]]).
2. **Product fix in `arks-server`** — align `write_octal` / header assembly with git v2.39.5 `prepare_header` field widths and device-field writes; add unit tests that compare **raw 512-byte headers** (and full stream) to fixtures captured in clean-room (not host Git).
3. **Capture fixtures once in Docker** — deterministic seed (fixed author/committer dates already used by `run-claimed-daemon-archive-e2e.sh`); store scrubbed golden `stock.tar` / first-header slices under `.spec/evidence/…` or `verify/fixtures/` **without** inventing Claimed until SoT row flips.
4. **Harness** — keep `dod-claimed-daemon-archive` / `run-claimed-daemon-archive-e2e.sh`:
   - structural/semantic probe → Partial signal
   - `cmp -s` → Claimed candidate only
   - default SKIP/Partial exit 0; `ARKS_DOD_STRICT=1` fail-closed until SoT + receipt
5. **Graduate** — only after clean-room receipt + explicit B5 SoT status amend (Spec-first). No invent Claimed from Partial or host `cargo test`.

## Docker compare harness design

| Piece | Role |
|-------|------|
| Venue | `arks-verify-m1` + lock stock Git **2.39.5**; Linux engine |
| Stock arm | `git daemon --enable=upload-archive` → `git archive --remote --format=tar` |
| Arks arm | `arks daemon --enable=upload-archive` → same client |
| Partial oracle | Python `tarfile` member tuple equality (name, type, mode, payload) |
| Claimed oracle | `cmp -s stock.tar arks.tar` (entire stream) |
| Classifier (debug) | First differing offset + field map (chksum@148, devmajor@329, …); **MUST NOT** auto-PASS |
| Receipts | `verify/out/claimed-daemon-archive/` during scaffold; scrubbed Claimed → `.spec/evidence/<id>/` |
| Honesty | Print Partial / Missing / SKIP; never print Claimed PASS until SoT row is Claimed |

Optional follow-on (non-blocking): compose service stays script-only (`dod-claimed-daemon-archive`); no empty `gates/dod-*` scaffold ([[sot:root/verify-directory-conventions]]).

## Acceptance tests (future Claimed)

| ID | Assert | Venue |
|----|--------|-------|
| B5-T1 | Stock Git in image == `git version 2.39.5` | Docker |
| B5-T2 | Structural/semantic streams equal | Docker (Partial retained if T3 fails) |
| B5-T3 | `cmp -s` stock vs arks ustar **PASS** | Docker — **Claimed gate** |
| B5-T4 | First-header bytes equal for `pax_global_header` + first file/dir members | Unit + Docker |
| B5-T5 | Device major/minor slots are seven ASCII `0` + NUL (git `"%07o"`), not all-NUL | Unit |
| B5-T6 | Chksum field is seven octal digits + NUL matching recomputed sum-with-spaces | Unit |
| B5-T7 | `ARKS_DOD_STRICT=1` exits non-zero while SoT B5 ≠ Claimed | Docker harness |
| B5-T8 | After SoT Claimed amend, STRICT green only with T3 receipt attached | Spec + Docker |

**Refuse as Claimed evidence:** host-only hex dumps; semantic-only greens; expanded protocol unit tests alone; opportunistic single `cmp` without pin + SoT amend.

## Cross-links

- Normative residual row: [[sot:root/arks-claimed-residual-gates]] B5
- Daemon intent honesty: [[sot:root/arks-daemon-product-intent]]
- Prior Docker shape research: [[research:root/20261111074523]]
- Partial evidence note: [[evidence:root/20261110232923]]

## Verification (this research)

1. Peer review of git v2.39.5 `prepare_header` citations vs arks `write_ustar_entry`.
2. No product Claimed language added; B5 remains Partial until implement + receipt wave.
3. Harness naming stays under [[sot:root/verify-directory-conventions]] (`dod-claimed-daemon-archive`).
