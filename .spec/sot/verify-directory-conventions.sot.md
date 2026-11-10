---
id: sot:root/verify-directory-conventions
title: Verify directory conventions and consistency
status: active
created: 2026-11-10T23:25:00Z
updated: 2026-11-11T08:45:00Z
tags: [verify, clean-room, docker, conventions, layout, naming, folders, filenames, skip, strict, matrices, claimed-residual, design, no-invent-claimed]
verify: manual
rel:
  depends_on:
    - research:root/20261111082440
    - research:root/20261110095355
    - sot:root/product-verification-clean-room
    - sot:root/structural-conventions
    - sot:root/verify-cleanroom-symmetry-matrix
    - sot:root/arks-claimed-residual-gates
    - sot:root/release-symmetry-precheck
    - sot:root/production-residual-register
    - sot:root/language-english-only
    - sot:root/public-surface
  evidence: []
  governs:
    - verify/README.md
    - verify/docker-compose.yml
    - verify/scripts/
    - verify/matrices/
    - verify/gates/
    - verify/fixtures/
    - verify/out/
    - verify/Dockerfile
    - verify/images.lock.json
---

# Verify directory conventions and consistency

Document revision **0.3.0** (SoT edit counter only — **not** a product/crate release). Normative **design** for how the product clean-room tree under `verify/` is laid out, named, and extended — including **internal folder naming** (0.2.0) and **filename naming** (this revision). Complements [[sot:root/product-verification-clean-room]] (authority) and [[sot:root/structural-conventions]] §7 (grouped overlay). Survey pain: [[research:root/20261111082440]].

**Honesty:** This SoT is **ops/harness convention** — **not** product Claimed. Scaffold / SKIP greens **MUST NOT** invent PASS/Claimed. English-only Specfold ([[sot:root/language-english-only]]). Document revision ≠ crates.io / `v*` product release.

## Rule

### 1. Directory tree contract

| Path | Owns | MUST NOT own |
|------|------|----------------|
| `verify/Dockerfile` | Clean-room image recipe | Host toolchains |
| `verify/images.lock.json` | Pin digests + expected stock Git / rustc / OpenSSH strings | Floating `latest` authority |
| `verify/docker-compose.yml` | Service → script entry map | Specfold SoTs |
| `verify/scripts/` | Deterministic entrypoints (`run-*.sh`, evaluators, host-advisory helpers) | Crate unit tests |
| `verify/matrices/` | Gate/milestone Markdown matrices + living symmetry CSVs | Product Claimed prose |
| `verify/gates/<id>/` | Per-gate oracles, locks, `out/` receipts (out-of-workspace Cargo when needed) | Default workspace membership |
| `verify/fixtures/` | Shared oracle intent/bytes (no secrets) | Crate-private `testdata/` |
| `verify/out/` | Cross-gate inventory / prerelease dumps (gitignored) | Per-gate Claimed receipts |
| `verify/README.md` | Operator map + authority reminder | Second product SoT |

Grouped layout is **mandatory** — no flat dump of unrelated `g*` crates or `matrix-*.md` at `verify/` root ([[research:root/20261110095355]], [[sot:root/structural-conventions]] rule 23).

```text
verify/
  Dockerfile
  docker-compose.yml
  images.lock.json
  README.md
  scripts/                 # compose argv entrypoints
  matrices/                # matrix-*.md + *-matrix.csv
  gates/<id>/              # gN | gN-<kebab> | fs*-advisory
  fixtures/                # shared oracle intent
  out/                     # gitignored cross-gate dumps
```

**Folder orthography (all of `verify/`):** directory names are **kebab-case ASCII** (lowercase letters, digits, hyphens). **MUST NOT** introduce `snake_case`, `PascalCase`, or camelCase folder names. **File** basenames follow §2a (kebab-case by default; Cargo / Rust module names follow Cargo norms). Rust crate layout under a gate (`src/`, `tests/`) follows Cargo norms (not kebab folders for modules).

### 1a. Top-level folder allow-list

| Name | Role | Notes |
|------|------|-------|
| `scripts/` | Compose argv entrypoints + host helpers | Only place for new `run-*.sh` / `*.ps1` by default |
| `matrices/` | Claim matrices + living CSVs | No gate oracle crates |
| `gates/` | Per-id oracle trees + gate-local `out/` | Ids per §1b |
| `fixtures/` | Shared oracle intent / bytes | Milestone subdirs kebab or short milestone ids (`m1/`); no secrets |
| `out/` | Cross-gate dumps (gitignored) | Topics kebab (`symmetry-inventory/`, `claimed-<concern>/`) |

**MUST:** new top-level directories under `verify/` require amending this SoT before landing.

**MUST NOT:** flat `g*` / `matrix-*` / `run-*` dumps at `verify/` root; Docker-related **directories** named `docker/`, `images/`, or `compose/` — keep recipe/lock/compose as **files** at `verify/` root (`Dockerfile`, `docker-compose.yml`, `images.lock.json`).

**Repo-root (not under `verify/`):** host Cargo dirs such as `target/`, `target-cleanroom/`, `target-pilot/` are **advisory host build roots**. They are **not** verify tree folders and **MUST NOT** be mirrored as committed `verify/target-*` trees. Clean-room Cargo uses compose named volume → `/cargo-target` (`CARGO_TARGET_DIR`).

### 1b. `gates/<id>/` id patterns

| Pattern | Example | Use |
|---------|---------|-----|
| `gN` | `g0` … `g41` | Primary product / milestone gate id (decimal, no leading zero except `g0`) |
| `gN-<kebab-suffix>` | `g8-live-scale`, `g8-local-forge`, `g28-offline-clones` | Variant / sibling oracle of the same G-series family |
| `fsNNN-<kebab>-advisory` | `fs007-host-advisory`, `fs014-lsm-advisory` | Host-advisory only (explicitly non-DoD) |
| `dod-<concern>` (reserved) | *(none under `gates/` at tip)* | **Reserved** if a residual/DoD slot needs a gate-local Cargo oracle; id **MUST** match compose service name |

**MUST:**

1. Gate directory basename **equals** the primary compose service name when a gate tree exists (`verify/gates/g28/` ↔ service `g28`).
2. New G-series variants use `gN-<kebab-suffix>` (hyphen before the suffix) — not jammed tokens.
3. New host-advisory gate dirs keep the `fsNNN-*-advisory` shape.

**MUST NOT:** invent a fourth id family (e.g. `gate-N/`, `B1/`, `claimed-*` without `dod-`) without amending this SoT + [[sot:root/product-verification-clean-room]].

**Legacy skew (allowed until Migration):** `g11l3` (compose + dir + `run-g11l3.sh`) — jammed L3 token; preferred form `g11-l3`. **Do not** rename in Spec-only waves; see Appendix A.

**DoD / Claimed residual without gate dirs:** most `dod-*` / `dod-claimed-*` services are **script-only** under `verify/scripts/` with optional receipts under `verify/out/<topic>/`. That is intentional — **MUST NOT** mass-create empty `gates/dod-*/` scaffolds.

### 1c. Nested folders inside `gates/<id>/`

| Child | Allowed? | Role |
|-------|----------|------|
| `out/` | **MUST** when the gate emits receipts | Gate-local receipts / metrics / locks (gitignored); runtime worktrees under `out/` OK |
| `src/`, `tests/` | Yes | Out-of-workspace Cargo oracle layout |
| `Cargo.toml`, `Cargo.lock` | Yes | Gate crate manifest (not workspace member) |
| `README.md`, `BUGSIM.md`, `*.md` | Yes | Operator / bugsim notes for that gate |
| `.gitkeep`, `.run.lock.dir/` | Yes | Placeholder / run lock |
| `httpd/` (or similar peer fixture dir) | Yes when colocated fixture peers need a tree (e.g. `g41/httpd/`) | Keep kebab; document in gate README / script header |
| Colocated `seed.sh` / tiny helpers | Yes sparingly | Prefer `verify/scripts/run-*.sh` for compose argv; gate-local scripts only when the helper is peer-specific (e.g. `g41/seed.sh`) |
| `fixtures/` under a gate | Discouraged | Prefer shared `verify/fixtures/` unless bytes are gate-private and documented |
| `scripts/` under a gate | Discouraged | Prefer `verify/scripts/` |
| `target/` under a gate | **MUST NOT commit** | Local/host cargo spill; prefer `CARGO_TARGET_DIR` / compose volume; gitignore if created |
| Nested second gate id as a **committed** sibling tree | **MUST NOT** | e.g. do not add `gates/g28/g28-offline-clones/`; variants are **siblings** under `gates/` |

Runtime scratch under `gates/<id>/out/**` (clones, worktrees) is ephemeral and **gitignored** — naming there is script-local, not a second public gate id.

### 1d. Consistency with compose service names

| Case | Rule |
|------|------|
| Gate has `verify/gates/<id>/` | Compose service name **MUST** be `<id>` (1:1) |
| Helper peers (`g41-seed`, `g41-dav`, `toolchain`) | **MAY** lack a `gates/<id>/` tree; names stay kebab and document the owning gate |
| `dod-*` / `dod-claimed-*` | Compose + `verify/scripts/` required; `gates/dod-*` only when an oracle crate / gate-local `out/` is actually needed |
| Volumes | Compose volume names (e.g. `arks-cargo-target`, `g41-git-data`) are kebab; they are **not** verify folders |

### 2. Service and script naming

| Family | Compose service | Script pattern | Notes |
|--------|-----------------|----------------|-------|
| Product / milestone gate | `gN` or `gN-<suffix>` | `run-gN.sh` / `run-gN-<suffix>.sh` | Gate dir `verify/gates/gN/` (or matching suffix) |
| Symmetry DoD pillar | `dod-<concern>-matrix` | `run-<concern>-…-matrix-e2e.sh` | Pillars: cli / daemon / hub |
| Symmetry orchestrator | `dod-symmetry` | `run-symmetry-dod.sh` (+ aliases `run-symmetry-{git,daemon,hub}.sh`) | Halt on hard FAIL only |
| Release precheck | `dod-symmetry-prerelease` | `run-symmetry-prerelease.sh` | [[sot:root/release-symmetry-precheck]] |
| Related hosting | `dod-self-host` | `run-self-host-e2e.sh` | Adjacent, **not** a fourth pillar |
| Claimed residual | `dod-claimed-<concern>` | `run-claimed-<concern>-e2e.sh` | Register under [[sot:root/arks-claimed-residual-gates]] |
| Claimed residual umbrella | `dod-claimed-residual` | `run-claimed-residual-dod.sh` | Orchestrates B-slots |
| Host advisory | `fsNNN-*-advisory` | `run-fsNNN-*-advisory.sh` | Explicitly non-DoD |
| Host wrappers | *(none required)* | `*.ps1` under `scripts/` | Drive compose only; never sole Verified judge |

**MUST:**

1. Every authoritative compose service names exactly one primary `verify/scripts/run-*.sh` (or a documented helper such as `verify/gates/g41/seed.sh` for seed peers).
2. New **DoD / residual / hosting** services use the `dod-` prefix; new **G-series** product gates keep `gN` / `gN-<suffix>`.
3. Script basenames are kebab-case ASCII; PowerShell helpers stay host-advisory.

**MUST NOT:** invent a third product-gate naming family without amending this SoT + [[sot:root/product-verification-clean-room]].

### 2a. Filename conventions (`verify/`)

Normative **file basename** rules. Aligns with compose service names, gate ids, and folder orthography from §1–§2 (0.2.0). Survey snapshot at tip ≥ `4a99f8e` (committed tree; sibling B-gate WIP may add dirty `run-claimed-*` / probes — do not treat dirty paths as Claimed).

#### 2a.1 Orthography (all new verify files)

| Rule | Norm |
|------|------|
| Case / separators | **kebab-case ASCII**: lowercase letters, digits, hyphens (`-`) |
| Extensions | Lowercase; one primary extension (`.sh`, `.ps1`, `.yml`, `.csv`, `.md`, `.json`, `.py`, `.txt`, `.out`, `.err`) |
| Underscores | **MUST NOT** in new harness / matrix / receipt basenames under `verify/` (exception: Cargo / Rust crate files per §2a.7) |
| Spaces / Unicode | **MUST NOT** |
| Dotfiles | Only established placeholders (`.gitkeep`, `.gitattributes`, `.run.lock.dir/` as a dir) — no new hidden authority files |

**MUST NOT (new work):** `snake_case` harness scripts (`*_eval.py` style), camelCase / PascalCase basenames, double extensions used as version tags (`receipt.json.bak`), or embedding host OS paths in filenames.

#### 2a.2 Shell scripts (`verify/scripts/` + gate-colocated)

| Kind | Pattern | Example |
|------|---------|---------|
| Compose primary entry | `run-<concern>.sh` | `run-g28.sh`, `run-claimed-tls-e2e.sh`, `run-symmetry-dod.sh` |
| Gate family | `run-gN.sh` / `run-gN-<kebab-suffix>.sh` | Matches §1b / compose service |
| Claimed residual | `run-claimed-<concern>-e2e.sh` | Concern matches `dod-claimed-<concern>` |
| Host advisory | `run-fsNNN-*-advisory.sh` | Matches gate / compose id |
| Symmetry / release / hosting | `run-<concern>.sh` as in §2 table | Prefer concern tokens already used by compose (`symmetry-prerelease`, `self-host-e2e`, …) |
| Non-compose helpers | `<verb>-<concern>.sh` **without** inventing a second compose family | `probe-*.sh`, `locate-arks.ps1` — document owner in header; **MUST NOT** be the sole Verified judge |
| PowerShell host drivers | `run-*.ps1` or `*-cleanroom-*.ps1` | Drive compose only |
| Gate-colocated peer helper | Short kebab or conventional short name | `gates/g41/seed.sh` — §1c; prefer `scripts/run-*.sh` for new compose argv |

**MUST:**

1. Authoritative compose argv scripts live under `verify/scripts/` and use the `run-` prefix unless an existing peer helper is already documented (`seed.sh`).
2. `<concern>` tokens are kebab-case and **match** the compose service stem where a 1:1 service exists (`g28-offline-clones` ↔ `run-g28-offline-clones.sh`; `dod-claimed-tls` ↔ `run-claimed-tls-e2e.sh`).
3. Evaluator / sidecar scripts invoked by a `run-*.sh` stay next to them under `scripts/` with kebab basenames going forward.

**MUST NOT:** place new `run-*.sh` at `verify/` root; use jammed gate tokens in new scripts (`run-g11l3.sh` is legacy only — Appendix A / B).

#### 2a.3 Matrix CSV and Markdown (`verify/matrices/`)

| Kind | Pattern | Notes |
|------|---------|-------|
| Milestone / gate claim Markdown | `matrix-<id>.md` | `matrix-m1.md`, `matrix-g28.md`, `matrix-journey.md` |
| Gate variant Markdown | `matrix-gN-<kebab>.md` | e.g. `matrix-g8-live-scale.md` |
| Living symmetry CSV | `<pillar>-<surface>-matrix.csv` or `<pillar>-<surface>-e2e-matrix.csv` | Tip: `cli-git-help-matrix.csv`, `daemon-git-arks-e2e-matrix.csv`, `hub-gh-arks-e2e-matrix.csv` |
| Design-only CSV | Prefer Specfold (`.spec/sot/…-matrix.csv`) | Living status stays under `verify/matrices/` — §6 |

**MUST:** new living CSVs end with `-matrix.csv` (optional `-e2e-` before `matrix` when the ledger is e2e-scoped). New claim Markdown keeps the `matrix-` prefix.

**MUST NOT:** drop living CSVs under `gates/*/`; use `Matrix.csv` / `MATRIX.md` / underscore CSV names for new files.

#### 2a.4 Receipt / evidence filenames (`out/`)

| Kind | Normative basename | Where |
|------|--------------------|-------|
| Primary gate receipt | `receipt.json` | `verify/gates/<id>/out/` |
| Primary metrics | `metrics.json` | Gate `out/` when the gate emits metrics |
| Human summary | `summary.txt` | Gate or cross-gate `out/<topic>/` |
| Concern-scoped receipt (multi-artifact gate) | `<concern>-receipt.json` / `<concern>-journeys.json` | e.g. `offline-clones-receipt.json` under `g28` |
| Host-advisory receipt | `receipt-host-advisory.json` or topic + `-host` | Label host in name; **MUST NOT** silently overwrite pin receipts |
| Cross-gate dumps | kebab topic files under `verify/out/<topic>/` | e.g. `untracked-cli.txt`, `summary.txt` |
| Scrubbed Claimed evidence | Specfold `.spec/evidence/<id>/` (tracked when scrubbed) | Not a substitute for gate `out/` during the run |

**SHOULD:** keep the **stable** names above for the latest clean-room result so operators and scripts can open a fixed path.

**MAY (debug / local only):** timestamp or short-sha suffixes (`receipt-<sha>.json`, `metrics-debug.json`) — treat as ephemeral; **MUST NOT** cite as sole Claimed authority when a stable `receipt.json` exists.

**MUST NOT:** invent a second stable public receipt name for the same gate without documenting it in the gate script header + this SoT; commit secrets or raw host home paths into receipt basenames.

#### 2a.5 Compose, README, lock, pin (verify root)

| File | Norm | Notes |
|------|------|-------|
| `docker-compose.yml` | **The** compose file name | **MUST NOT** add a second root compose (`compose.yml`, `docker-compose.yaml`) without amending this SoT |
| `Dockerfile` | Image recipe | Single root recipe |
| `images.lock.json` | Pin digests + expected stock strings | Authority for clean-room pin |
| `README.md` | Operator map | Cross-links this SoT; English-only |
| `.gitattributes` | Line-ending / export attrs | OK at verify root |

Helper compose overrides **MAY** exist only if documented in `README.md` and clearly non-authoritative; default operator path remains `verify/docker-compose.yml`.

#### 2a.6 Fixtures and other non-Cargo files

| Kind | Pattern |
|------|---------|
| Shared fixture docs / bytes | kebab or short milestone paths (`m1/blob-hello.md`); no secrets |
| Python helpers under `scripts/` or `fixtures/` | New files **kebab-case** (`.py`); legacy snake_case listed in Appendix B |
| Gate notes | `README.md`, `BUGSIM.md`, optional `README-<concern>.md` (kebab concern) |

#### 2a.7 Exceptions (Cargo / Rust — not kebab-forced)

| Path | Rule |
|------|------|
| `Cargo.toml`, `Cargo.lock` | Cargo spelling |
| `src/lib.rs`, `tests/*.rs` | Rust module / test file norms (`snake_case.rs` **allowed** here only) |
| Gate crate package names inside manifests | Follow Cargo; directory id stays kebab per §1b |

#### 2a.8 Alignment checklist (compose ↔ script ↔ gate ↔ matrix)

When adding a gate or residual slot, **all** of the following that exist **MUST** share the same kebab concern token:

1. Compose service name (`gN-…` / `dod-…` / `fsNNN-…`)
2. `verify/scripts/run-….sh` basename stem (§2 / §2a.2)
3. `verify/gates/<id>/` directory basename (§1b) when a gate tree exists
4. Matrix file stem where a dedicated matrix exists (`matrix-<id>.md`)
5. Cross-gate out topic when used (`verify/out/claimed-<concern>/`)

Folder rules remain §1a–§1d; this subsection only freezes **filenames**.

### 3. Artifact output locations

| Kind | Normative path | Git |
|------|----------------|-----|
| Per-gate receipts / metrics / locks | `verify/gates/<id>/out/` (+ `.run.lock.dir/` when used) | **gitignored** (`.gitignore`) |
| Cross-gate symmetry / prerelease inventory | `verify/out/<topic>/` (e.g. `symmetry-prerelease`, `symmetry-inventory`) | **gitignored** |
| Scrubbed Claimed / Verified evidence | `.spec/evidence/<id>/` | Tracked when scrubbed |
| Host-only exploratory dumps | Prefer `verify/out/<topic>-host/` or `.spec.local/` | Never cite as Verified |

**MUST:**

1. Authoritative Docker runs write receipts under the paths above (or env override documented in the script header, e.g. `ARKS_INVENTORY_OUT`).
2. Windows hosts create `out/` directories before Tee/redirect wrappers when the host shell creates the file (missing dir aborts before Docker evidence lands — see `verify/README.md`).
3. Host inventory copies **MUST** be labeled host / advisory in path or filename; they **MUST NOT** overwrite clean-room pin dumps silently.

### 4. SKIP / scaffold vs STRICT fail-closed

| Mode | Env | Expected behavior |
|------|-----|-------------------|
| Default scaffold | `ARKS_DOD_STRICT` unset or `0` | Unfinished DoD / residual / hosting slots **MAY** exit 0 with explicit `SKIP` / `scaffold` language |
| Fail-closed | `ARKS_DOD_STRICT=1` | Unfinished must-implement / Claimed rows **MUST** exit non-zero — **no invent PASS** |
| Release hub Missing policy | `ARKS_RELEASE_BLOCK_HUB_MISSING` | Default `0` = hub v1 Missing advisory; `1` = block (release SoT) |

**ABSOLUTE:**

1. Smoke / SKIP ≠ Claimed ([[sot:root/verify-cleanroom-symmetry-matrix]], [[sot:root/arks-claimed-residual-gates]]).
2. Host cargo greens are advisory only ([[sot:root/product-verification-clean-room]]).
3. New scaffold scripts **MUST** print SKIP/scaffold (or equivalent honesty) on the default path and honor `ARKS_DOD_STRICT=1`.

### 5. Clean-room pin (normative) vs host (non-normative)

| Surface | Normative judge | Host role |
|---------|-----------------|-----------|
| Stock Git / git-daemon | Clean-room pin **`git version 2.39.5`** (`verify/images.lock.json`) | Inventory / skew only (e.g. host 2.56) — **MUST NOT** override pin DoD |
| Image / rustc / OpenSSH | `arks-verify-m1:0.1.0` + lock expected strings | Advisory rebuild convenience |
| Engine | Linux containers; on Windows Desktop **desktop-linux** | Host PowerShell drives compose only |
| Cargo target | Compose named volume → `/cargo-target` (`CARGO_TARGET_DIR`) | Host `target/` / `target/*.exe` advisory |

Advancing the Git pin remains [[sot:root/clean-room-git-pin-advance]]. Latest-inventory continuity remains [[sot:root/release-symmetry-precheck]] — latest green ≠ pin Claimed.

### 6. Matrix CSV / Markdown placement vs gates

| Artifact | Placement | Owning SoT (examples) |
|----------|-----------|------------------------|
| Milestone / gate claim matrices | `verify/matrices/matrix-m*.md`, `matrix-g*.md`, `matrix-journey.md`, … | Gate / milestone SoTs |
| Living symmetry CSVs | `verify/matrices/*-matrix.csv` (cli / daemon / hub status ledgers) | Pillar SoTs + [[sot:root/verify-cleanroom-symmetry-matrix]] |
| Hub design leaf CSV | `.spec/sot/arks-hub-gh-command-matrix.csv` (design inventory) | [[sot:root/arks-hub-gh-command-matrix]] — living status still under `verify/matrices/` |
| Per-gate oracle code | `verify/gates/<id>/` | Gate SoT / plan; **not** under `matrices/` |

**MUST:** matrices describe claims; gates execute oracles; Specfold SoTs own normative meaning. Do not bury living CSVs under `gates/*/`.

### 7. How new Claimed residual gates register

When adding a residual Claimed slot (B-series or successor):

1. **Spec first** — amend [[sot:root/arks-claimed-residual-gates]] (acceptance + honesty) **before** or with the harness commit; do not invent Claimed from green SKIP.
2. **Compose + script names** — `dod-claimed-<concern>` → `verify/scripts/run-claimed-<concern>-e2e.sh`; wire into `dod-claimed-residual` orchestrator.
3. **Default SKIP / STRICT** — honor §4; print design SoT path in script header.
4. **Receipts** — gate-local under `verify/gates/…/out/` **or** documented `verify/out/claimed-<concern>/` while scaffold; scrubbed Claimed evidence → `.spec/evidence/<id>/`.
5. **Cross-links** — residual register next-queue + [[sot:root/verify-cleanroom-symmetry-matrix]] adjacent note + `.spec/README` agent entry if operators need discovery.
6. **Coordinate** — do not clobber sibling in-flight B-gate scripts; prefer Spec-only commits until harness ownership is clear.

Related hosting stays `dod-self-host` ([[sot:root/arks-self-host-server]]) — complementary, **not** a substitute for B1/B2/B7/B9.

### 8. Relationship map (do not collapse)

| Concern | SoT | Compose anchors |
|---------|-----|-----------------|
| Clean-room authority | [[sot:root/product-verification-clean-room]] | `gN`, image lock |
| Tree grouping / Cargo boundary | [[sot:root/structural-conventions]] §7 + this SoT | `scripts/` `matrices/` `gates/` `fixtures/` |
| Three symmetry pillars | [[sot:root/verify-cleanroom-symmetry-matrix]] | `dod-*-matrix`, `dod-symmetry` |
| Release pin / untracked precheck | [[sot:root/release-symmetry-precheck]] | `dod-symmetry-prerelease` |
| Claimed residual B-slots | [[sot:root/arks-claimed-residual-gates]] | `dod-claimed-*` |
| Honesty ledger | [[sot:root/production-residual-register]] | *(citations only)* |
| Directory consistency (this doc) | **this SoT** | folder + **filename** naming / out / SKIP / registration |

## Appendix A — Forbidden / legacy aliases and recommended migration (folders)

**Convention SoT first.** This appendix lists **folder / id** renames for a **future harness wave**. Spec-only commits **MUST NOT** mass-rename the tree unless a row is marked **MUST** and the change is tiny and coordinated. Sibling B-gate WIP **MUST NOT** be clobbered.

### Forbidden going forward (new work)

| Form | Why |
|------|-----|
| New top-level dirs under `verify/` outside §1a allow-list | Breaks grouped overlay |
| New jammed G-ids (`g11l3`-style) | Prefer `gN-<kebab>` |
| `snake_case` / camelCase verify folders | Orthography lock |
| Committed `gates/<id>/target/` or `verify/target-*` | Host/clean-room Cargo skew |
| Empty `gates/dod-*/` scaffolds “for symmetry” | Script-first DoD/residual until oracle needed |
| Flat `g*` / `matrix-*` at `verify/` root | [[sot:root/structural-conventions]] rule 23 |

### Legacy aliases (grandfathered)

| Current | Preferred | Status |
|---------|-----------|--------|
| `gates/g11l3/` + compose `g11l3` + `run-g11l3.sh` | `g11-l3` everywhere (dir, compose, script, matrix rows) | **Legacy** — Claimed evidence cites `g11l3`; rename only in a dedicated harness PR with matrix/evidence path notes |
| Repo-root `target-cleanroom/`, `target-pilot/` | Host advisory only; clean-room uses compose volume | **Keep** as host conveniences; do not promote into `verify/` |
| Gate-local `g41/seed.sh` | Acceptable peer helper | **Keep** unless folded into `scripts/` with compose update |

### Recommended renames (non-MUST — Migration backlog)

| From | To | Notes |
|------|----|-------|
| `verify/gates/g11l3/` | `verify/gates/g11-l3/` | Update compose service, `run-g11l3.sh` → `run-g11-l3.sh`, `matrix-l3.md` rows, and cite legacy alias in evidence indexes — **do not** rewrite historical evidence blobs |
| *(none else required at tip for folders)* | | Sibling dirs `g8-live-scale`, `g8-local-forge`, `g28-offline-clones` already match `gN-<kebab>`; `fs*-advisory` already compliant |

**Survey snapshot (folders, tip ≥ `3f1a484`):** top-level = `scripts/` `matrices/` `gates/` `fixtures/` `out/` only; gate nested children observed = `out/` `src/` `tests/` `httpd/` (+ Cargo files); no `dod-*` dirs under `verify/`; host `target*` at repo root only (plus ephemeral `gates/g1/target` spill).

## Appendix B — Filename skew survey and recommended migration

**Recommend only** unless a row is marked trivial **MUST**. Do **not** rename in Spec-only waves; do **not** clobber dirty B-gate WIP scripts.

### Forbidden going forward (new filenames)

| Form | Why |
|------|-----|
| New `snake_case` / camelCase harness or matrix basenames under `verify/` (non-Cargo) | §2a.1 orthography |
| Second root compose filename (`compose.yml`, `docker-compose.yaml`) | §2a.5 single authority file |
| New jammed script ids (`run-g11l3.sh`-style) | Prefer `run-gN-<kebab>.sh` |
| New stable receipt names that bypass `receipt.json` / documented `<concern>-receipt.json` | Breaks operator fixed paths |
| Underscore Python / shell helpers as new compose judges | Prefer kebab; helpers must not sole-judge Verified |

### Legacy / skew filenames (grandfathered until Migration)

| Current | Preferred | Status |
|---------|-----------|--------|
| `verify/scripts/run-g11l3.sh` | `run-g11-l3.sh` | **Legacy** — couples to Appendix A `g11l3` rename |
| `verify/scripts/g19_eval.py` | `g19-eval.py` (or fold into `run-g19.sh` only) | **Skew** — underscore; rename with `run-g19.sh` argv update |
| `verify/fixtures/openai_compat_fixture.py` | `openai-compat-fixture.py` | **Skew** — snake_case fixture helper; rename when callers updated |
| `verify/matrices/matrix-l3.md` | `matrix-g11-l3.md` (after id unjam) **or** keep as L3 milestone alias with README note | **Advisory** — milestone “L3” token predates `g11-l3` |
| `verify/matrices/matrix-l8-e2e.md` / `run-l8-e2e.sh` | Keep L8 milestone names **or** document as milestone alias beside G-series | **Keep** unless milestone SoT retires L8 label |
| `verify/matrices/matrix-s6-symmetry.md` / `run-s6-symmetry.sh` | Prefer `symmetry` / DoD family names already in §2 | **Advisory** — S6 historical label |
| `verify/scripts/probe-*.sh` | Keep as non-`run-` helpers | **Keep** — not compose primary judges |
| `gates/g41/seed.sh` | Keep short colocated name | **Keep** — §1c / Appendix A |
| Ephemeral `out/**/receipt-<sha>.json`, `metrics-debug.json`, `receipt-fail1.json`, … | Prefer overwrite stable `receipt.json` / `metrics.json`; keep sha dumps local-only | **Keep pattern for debug**; do not promote to Claimed citations |

### Recommended renames (non-MUST — filename Migration backlog)

| From | To | Notes |
|------|----|-------|
| `scripts/g19_eval.py` | `scripts/g19-eval.py` | Update `run-g19.sh` / compose references; trivial when no parallel WIP on g19 |
| `fixtures/openai_compat_fixture.py` | `fixtures/openai-compat-fixture.py` | Update importers; coordinate with Claimed agent-llm WIP if dirty |
| `scripts/run-g11l3.sh` | `scripts/run-g11-l3.sh` | **Only** with Appendix A folder/compose rename |
| Stable multi-artifact names already compliant | *(none)* | `offline-clones-receipt.json`, living `*-matrix.csv`, root `docker-compose.yml` / `images.lock.json` / `README.md` match §2a |

**Survey snapshot (filenames, tip ≥ `4a99f8e`):** root files = `Dockerfile` `docker-compose.yml` `images.lock.json` `README.md` `.gitattributes`; scripts dominated by `run-*.sh` + few `*.ps1` / `g19_eval.py` / `probe-*.sh`; matrices = `matrix-*.md` + three `*-matrix.csv`; primary receipts = `receipt.json` / `metrics.json` / `summary.txt` with local sha/debug variants under gate `out/`; compose filename singular.

## Why

`verify/` accumulated G-gates, symmetry DoD, release precheck, self-host scaffold, and Claimed residual slots under one tree. Without a single convention SoT, agents duplicate out paths, mix host and pin authority, invent folder or **file** orthography, or register new residuals without Spec/residual cross-links. This document freezes the directory, **folder-naming**, and **filename-naming** contract so Spec-first waves stay consistent and fail-closed.

## Verification

1. `spec check` clean for this SoT and [[research:root/20261111082440]] when Spec CLI is available; otherwise peer review of links.
2. Cross-links from [[sot:root/verify-cleanroom-symmetry-matrix]], [[sot:root/arks-claimed-residual-gates]], [[sot:root/release-symmetry-precheck]], [[sot:root/production-residual-register]], [[sot:root/product-verification-clean-room]], [[sot:root/structural-conventions]], and `.spec/README.md`.
3. `verify/README.md` English map matches §1–§5 and folder/filename pointers (§1a–§1d / §2a / Appendix A–B).
4. Clean-room pin check: container reports stock Git matching `verify/images.lock.json` (`git version 2.39.5`).
5. No product Claimed language added to public `docs/` from this design-only wave.
6. No mass rename of `gates/` or harness filenames in the Spec-only wave that lands §1a–§1d / §2a — Migration backlog only unless a tiny MUST row is explicitly executed.
