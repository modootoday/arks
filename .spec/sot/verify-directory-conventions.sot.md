---
id: sot:root/verify-directory-conventions
title: Verify directory conventions and consistency
status: active
created: 2026-11-10T23:25:00Z
updated: 2026-11-11T08:50:00Z
tags: [verify, clean-room, docker, conventions, layout, naming, folders, filenames, numeric, zero-pad, skip, strict, matrices, claimed-residual, design, no-invent-claimed]
verify: manual
rel:
  depends_on:
    - research:root/20261111085000
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

Document revision **0.5.0** (SoT edit counter only — **not** a product/crate release). Normative **design** for how the product clean-room tree under `verify/` is laid out, named, and extended — including **internal folder naming** (0.2.0), **filename naming** (0.3.0), **numeric notation** (0.4.0 wrote unpadded G-series; **this revision supersedes** unpadded `gN` **and** any interim two-digit `gNN` guidance with **three-digit zero-padded `gNNN`**, same width class as `fsNNN`), and **contextual G-gate ids** (`gNNN-<kebab-context>`). Complements [[sot:root/product-verification-clean-room]] (authority) and [[sot:root/structural-conventions]] §7 (grouped overlay). Survey pain: [[research:root/20261111082440]].

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
  gates/<id>/              # gNNN | gNNN-<kebab-context> | fs*-advisory
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
| `gNNN` | `g000` … `g999` | Primary product / milestone gate id — **three-digit zero-padded** (same width class as `fsNNN`) |
| `gNNN-<kebab-context>` | `g015-smart-http`, `g032-git-daemon-wire`, `g008-live-scale` | **Required for new gates** — short English concern slug so `verify/gates/` browsing conveys intent; also used for variants / siblings |
| `fsNNN-<kebab>-advisory` | `fs007-host-advisory`, `fs014-lsm-advisory` | Host-advisory only (explicitly non-DoD); **unchanged** three-digit |
| `dod-<concern>` (reserved) | *(none under `gates/` at tip)* | **Reserved** if a residual/DoD slot needs a gate-local Cargo oracle; id **MUST** match compose service name |

**MUST:**

1. Gate directory basename **equals** the primary compose service name when a gate tree exists (after Migration: `verify/gates/g028-live-private-arks/` ↔ service `g028-live-private-arks`; live tip still uses unpadded bare ids until Migration).
2. **New** G-series gate dirs use `gNNN-<kebab-context>` (three-digit `NNN` + hyphen + short English concern slug) — not bare `gNNN`, not unpadded `gN`, not two-digit `gNN`, and not jammed tokens (`g11l3`).
3. New host-advisory gate dirs keep the `fsNNN-*-advisory` shape (three-digit `NNN`).
4. Compose service, `run-*.sh` stem, and dedicated `matrix-*.md` stem **MUST** use the **same** contextual id string when those artifacts exist (§1d / §2a.8 / §2b.7).

**MUST NOT:** invent a fourth id family (e.g. `gate-N/`, `B1/`, `claimed-*` without `dod-`) without amending this SoT + [[sot:root/product-verification-clean-room]].

**Legacy (grandfathered until Migration):** bare unpadded `g0`…`g41`, any two-digit-only `gNN` forms, and existing hyphen variants that still use unpadded digits (`g8-live-scale`, `g11-l3`, `g28-offline-clones`). Prefer Appendix C / D renames. **Hyphen unjam already migrated:** `g11l3` → `g11-l3` (compose + dir + `run-g11-l3.sh`); three-digit pad + context still backlog. Historical evidence blobs may still cite `g11l3` / unpadded ids.

**DoD / Claimed residual without gate dirs:** most `dod-*` / `dod-claimed-*` services are **script-only** under `verify/scripts/` with optional receipts under `verify/out/<topic>/`. That is intentional — **MUST NOT** mass-create empty `gates/dod-*/` scaffolds.

### 1c. Nested folders inside `gates/<id>/`

| Child | Allowed? | Role |
|-------|----------|------|
| `out/` | **MUST** when the gate emits receipts | Gate-local receipts / metrics / locks (gitignored); runtime worktrees under `out/` OK |
| `src/`, `tests/` | Yes | Out-of-workspace Cargo oracle layout |
| `Cargo.toml`, `Cargo.lock` | Yes | Gate crate manifest (not workspace member) |
| `README.md`, `BUGSIM.md`, `*.md` | Yes | Operator / bugsim notes for that gate |
| `.gitkeep`, `.run.lock.dir/` | Yes | Placeholder / run lock |
| `httpd/` (or similar peer fixture dir) | Yes when colocated fixture peers need a tree (e.g. live `g41/httpd/`; prefer `g041-…/httpd/` after Migration) | Keep kebab; document in gate README / script header |
| Colocated `seed.sh` / tiny helpers | Yes sparingly | Prefer `verify/scripts/run-*.sh` for compose argv; gate-local scripts only when the helper is peer-specific (e.g. live `g41/seed.sh`; prefer matching `g041-…` after Migration) |
| `fixtures/` under a gate | Discouraged | Prefer shared `verify/fixtures/` unless bytes are gate-private and documented |
| `scripts/` under a gate | Discouraged | Prefer `verify/scripts/` |
| `target/` under a gate | **MUST NOT commit** | Local/host cargo spill; prefer `CARGO_TARGET_DIR` / compose volume; gitignore if created |
| Nested second gate id as a **committed** sibling tree | **MUST NOT** | e.g. do not add `gates/g028/g028-offline-clones/`; variants are **siblings** under `gates/` |

Runtime scratch under `gates/<id>/out/**` (clones, worktrees) is ephemeral and **gitignored** — naming there is script-local, not a second public gate id.

### 1d. Consistency with compose service names

| Case | Rule |
|------|------|
| Gate has `verify/gates/<id>/` | Compose service name **MUST** be `<id>` (1:1) |
| Helper peers (`g41-seed` / future `g041-…-seed`, `g41-dav`, `toolchain`) | **MAY** lack a `gates/<id>/` tree; names stay kebab, document the owning gate, and **align digits** with the owning `gNNN` when migrated |
| `dod-*` / `dod-claimed-*` | Compose + `verify/scripts/` required; `gates/dod-*` only when an oracle crate / gate-local `out/` is actually needed |
| Volumes | Compose volume names (e.g. `arks-cargo-target`, `g41-git-data` / future `g041-…-git-data`) are kebab; they are **not** verify folders |

### 2. Service and script naming

| Family | Compose service | Script pattern | Notes |
|--------|-----------------|----------------|-------|
| Product / milestone gate | `gNNN-<kebab-context>` (new); legacy bare `gN` / unpadded variants grandfathered | `run-gNNN-<kebab-context>.sh` | Gate dir matches id 1:1; digit width §2b |
| Symmetry DoD pillar | `dod-<concern>-matrix` | `run-<concern>-…-matrix-e2e.sh` | Pillars: cli / daemon / hub |
| Symmetry orchestrator | `dod-symmetry` | `run-symmetry-dod.sh` (+ aliases `run-symmetry-{git,daemon,hub}.sh`) | Halt on hard FAIL only |
| Release precheck | `dod-symmetry-prerelease` | `run-symmetry-prerelease.sh` | [[sot:root/release-symmetry-precheck]] |
| Related hosting | `dod-self-host` | `run-self-host-e2e.sh` | Adjacent, **not** a fourth pillar |
| Claimed residual | `dod-claimed-<concern>` | `run-claimed-<concern>-e2e.sh` | Register under [[sot:root/arks-claimed-residual-gates]] |
| Claimed residual umbrella | `dod-claimed-residual` | `run-claimed-residual-dod.sh` | Orchestrates B-slots |
| Host advisory | `fsNNN-*-advisory` | `run-fsNNN-*-advisory.sh` | Explicitly non-DoD |
| Host wrappers | *(none required)* | `*.ps1` under `scripts/` | Drive compose only; never sole Verified judge |

**MUST:**

1. Every authoritative compose service names exactly one primary `verify/scripts/run-*.sh` (or a documented helper such as live `verify/gates/g41/seed.sh` for seed peers).
2. New **DoD / residual / hosting** services use the `dod-` prefix; new **G-series** product gates use `gNNN-<kebab-context>` (§1b / §2b) — never unpadded `gN`, never two-digit-only `gNN` as the normative form.
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
| Gate family | `run-gNNN-<kebab-context>.sh` (new); legacy `run-gN.sh` grandfathered | Matches §1b / §2b / compose service |
| Claimed residual | `run-claimed-<concern>-e2e.sh` | Concern matches `dod-claimed-<concern>` |
| Host advisory | `run-fsNNN-*-advisory.sh` | Matches gate / compose id |
| Symmetry / release / hosting | `run-<concern>.sh` as in §2 table | Prefer concern tokens already used by compose (`symmetry-prerelease`, `self-host-e2e`, …) |
| Non-compose helpers | `<verb>-<concern>.sh` **without** inventing a second compose family | `probe-*.sh`, `locate-arks.ps1` — document owner in header; **MUST NOT** be the sole Verified judge |
| PowerShell host drivers | `run-*.ps1` or `*-cleanroom-*.ps1` | Drive compose only |
| Gate-colocated peer helper | Short kebab or conventional short name | live `gates/g41/seed.sh` — §1c; prefer `scripts/run-*.sh` for new compose argv |

**MUST:**

1. Authoritative compose argv scripts live under `verify/scripts/` and use the `run-` prefix unless an existing peer helper is already documented (`seed.sh`).
2. `<concern>` tokens are kebab-case and **match** the compose service stem where a 1:1 service exists (`g028-offline-clones` ↔ `run-g028-offline-clones.sh` (live still `g28-offline-clones` until Migration); `dod-claimed-tls` ↔ `run-claimed-tls-e2e.sh`).
3. Evaluator / sidecar scripts invoked by a `run-*.sh` stay next to them under `scripts/` with kebab basenames going forward.

**MUST NOT:** place new `run-*.sh` at `verify/` root; use jammed gate tokens in new scripts (`run-g11l3.sh` is legacy only — Appendix A / B).

#### 2a.3 Matrix CSV and Markdown (`verify/matrices/`)

| Kind | Pattern | Notes |
|------|---------|-------|
| Milestone / gate claim Markdown | `matrix-<id>.md` | `matrix-m1.md`, live `matrix-g28.md` → prefer `matrix-g028-….md` after Migration, `matrix-journey.md` |
| Gate / contextual Markdown | `matrix-gNNN-<kebab-context>.md` | e.g. prefer `matrix-g008-live-scale.md` (live `matrix-g8-live-scale.md` grandfathered) |
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

1. Compose service name (`gNNN-…` / `dod-…` / `fsNNN-…`)
2. `verify/scripts/run-….sh` basename stem (§2 / §2a.2)
3. `verify/gates/<id>/` directory basename (§1b) when a gate tree exists
4. Matrix file stem where a dedicated matrix exists (`matrix-<id>.md`)
5. Cross-gate out topic when used (`verify/out/claimed-<concern>/`)

Folder rules remain §1a–§1d; this subsection only freezes **filenames**.

### 2b. Numeric notation conventions

Normative **digit width / zero-padding** for ids that appear in gate folders, compose services, scripts, matrices, and Specfold evidence. Complements §1b (id patterns) and §2a (filenames). Survey snapshot at tip ≥ `9b27ea5` (committed tree; sibling B-gate / verify-migration WIP may add dirty paths — do not treat dirty paths as Claimed; do not fight mid-rename siblings).

**Supersession:** Revision **0.4.0** required **unpadded** G-series (`g1`, `g11`) and forbade zero-padding for lexical sort. Interim chat guidance that preferred two-digit `gNN` (`g01`…`g99`) is also **withdrawn**. Normative G-series width is now **three-digit `gNNN`**, the **same width class as `fsNNN`**, so GitHub UI / lexicographic folder sort matches numeric order (`g001`…`g041`, not `g1` then `g10` then `g11` then `g2`).

#### 2b.1 Survey — families in use

| Family | Shape at tip (live) | Normative width / pad | Examples (normative → Migration) |
|--------|---------------------|------------------------|----------------------------------|
| G-series product / milestone gates | `g` + **unpadded** decimal (legacy) | Fixed width **3**, zero-pad (`gNNN`) | live `g1` / `g11` / `g41` → `g001` / `g011` / `g041` (+ context slug for **new** / preferred Migration) |
| G-series contextual / variants | live `gN-<kebab>` unpadded | Same three-digit `NNN` + `-<kebab-context>` | live `g8-live-scale` → `g008-live-scale`; new e.g. `g032-git-daemon-wire` |
| Legacy jammed G-token | `g11l3` | Prefer hyphen + later `g011-…` | `g11-l3` migrated; pad/context backlog — Appendix A / C |
| Host-advisory FS slots | `fs` + **three-digit** + `-<kebab>-advisory` | Fixed width **3**, zero-pad (**unchanged**) | `fs007-host-advisory`, `fs014-lsm-advisory` |
| Milestone matrix ids | `matrix-mN.md` | Unpadded `N` (**not** a G/FS pad family) | `matrix-m1.md` … `matrix-m11.md` |
| Gate claim matrices | live `matrix-gN.md` / `matrix-gN-<kebab>.md` | Mirror live gate id until Migration; then `matrix-gNNN-<kebab-context>.md` | `matrix-g19.md` → prefer `matrix-g019-comparative-performance.md` |
| Milestone aliases (historical) | `matrix-l3.md`, `matrix-l8-e2e.md`, `matrix-s6-symmetry.md` | Letter + historical label | Not a pad family — Appendix B |
| Compose services | Same token as gate / DoD / FS id | Inherit family pad rule | live `g11`, `g11-l3`, `fs007-host-advisory`, `dod-claimed-tls` |
| Specfold evidence dirs | `YYYYMMDDHHmmss-<kebab>` | **14-digit** compact UTC timestamp, then kebab | `20261110231959-claimed-agent-llm` |
| DoD / Claimed residual | `dod-<concern>` / `dod-claimed-<concern>` | **No** series number pad (kebab concerns) | `dod-symmetry`, `dod-claimed-ssh` |

**Observed G range (committed, live unpadded):** `g0`–`g8`, `g10`–`g41` (no `g9` dir at tip). **FS range:** `fs007`, `fs014` only.

#### 2b.2 Width rules (choose one family; do not mix)

| Width class | Digits | Pad? | Use under `verify/` / Specfold |
|-------------|--------|------|--------------------------------|
| **Unpadded decimal** | `0`…∞ as needed | **MUST NOT** zero-pad | Milestone `mN` / `matrix-mN.md` only (and historical L/S aliases) — **not** G-series |
| **Two-digit fixed** `01`–`99` | Always 2 | Zero-pad to 2 | **MUST NOT** use for G-series. Reserved only if a future non-G series is explicitly declared here |
| **Three-digit fixed** `001`–`999` | Always 3 | Zero-pad to 3 | **G-series `gNNN`** and **host-advisory `fsNNN`** — always `g011` / `fs007`, never `g11` / `g0110` / `fs7` / `fs07` as normative new ids |
| **Timestamp fixed** | 14 (`YYYYMMDDHHmmss`) | Calendar zero-fill by format | `.spec/evidence/<id>/` directory prefix only — **not** a gate / compose series |

**Prose shorthand:** In this SoT, `gNNN` means “`g` + **exactly three** zero-padded decimal digits.” Informal unpadded `g11` / two-digit `g11` in historical prose cites the **live** grandfathered token until Migration rewrites paths.

#### 2b.3 Gate ids — `gNNN` + contextual kebab

| Form | Norm | Example |
|------|------|---------|
| `gNNN-<kebab-context>` | **Required for new gates** — three-digit number + short English concern slug | `g015-smart-http`, `g032-git-daemon-wire` |
| `gNNN` (bare) | Allowed only as **legacy / Migration intermediate**; prefer contextual form when renaming | `g011` (prefer `g011-lease-product`) |
| Live unpadded `gN` / `gN-<kebab>` | **Grandfathered** until Migration | `g11`, `g8-live-scale`, `g28-offline-clones` |
| Two-digit-only `gNN` as a **normative** family | **MUST NOT** | Do not stop at `g01`…`g99`; use `g001`…`g999` |
| Unpadded `gN` as a **normative** family | **MUST NOT** (supersedes 0.4.0) | Do not add new `g1` / `g42` without three-digit pad + context |
| Jammed digit+letter | **MUST NOT** for new work | `g11l3` legacy only → hyphen then pad |

**MUST:**

1. New G-series ids use **three-digit** zero-padded decimals — `g001` not `g1` / `g01`; `g011` not `g11` / `g0110`.
2. New G-series dirs / compose / scripts / matrices use `gNNN-<kebab-context>` — never bare `gNNN` for **new** work.
3. Variant / sibling suffixes use a hyphen after the three-digit number: `gNNN-<kebab-context>` — never jam letters onto digits (`g11l3` is legacy only).
4. The **same id string** appears in folder basename, compose service name, `run-*.sh` stem, and `matrix-*.md` stem when those artifacts exist.

**MUST NOT:**

1. Introduce new **unpadded** G-series ids (`g1`, `g42`).
2. Introduce new **two-digit-only** padded G-series ids (`g01`…`g99`) as the normative width — always use three digits (`g001`…`g099`).
3. Mix widths for the same logical gate (`gates/g11/` + `run-g011.sh`) during or after Migration.
4. Re-widen beyond three digits for G-series (`g0011`) — range is `000`–`999` in this SoT; amending required above that.

#### 2b.4 When **not** to zero-pad

| Context | Rule |
|---------|------|
| Milestone `mN` / `matrix-mN.md` | Unpadded (`m1`, `m11` — not `m001`) |
| DoD / Claimed concern tokens | No numeric pad (`claimed-tls`, not `claimed-001-tls`) |
| Document revision / crate semver | Semver rules — **out of scope** for this subsection |
| Human prose citing **live** paths | Prefer the token that currently exists on disk until Migration (`g11`, not `g011`) |
| Historical evidence directory names | **Do not** rewrite committed evidence basenames solely to re-pad |

Zero-pad **for** fixed-width series declared here: **`gNNN`**, **`fsNNN`**, and evidence `YYYYMMDDHHmmss`.

#### 2b.5 `fsNNN` (three-digit denser series) — unchanged

| Rule | Norm |
|------|------|
| Width | Always **3** decimal digits after `fs` |
| Range | `001`–`999` (tip uses sparse ids `007`, `014`) |
| Suffix | `-<kebab>-advisory` on dirs / compose / scripts |
| Script | `run-fsNNN-*-advisory.sh` matching the id |
| Why fixed-3 | Host-advisory slots are a denser, non-G namespace; lexical order ≈ numeric order within 001–999 |
| Relation to G | **Same digit width class** as `gNNN` — operators learn one pad rule for numbered verify slots |

**MUST NOT:** introduce `fs7`, `fs07`, or a second advisory family with a different pad width without amending this SoT.

#### 2b.6 Evidence timestamps

| Rule | Norm |
|------|------|
| Prefix | `YYYYMMDDHHmmss` (14 digits, UTC wall components zero-filled by the timestamp format) |
| Suffix | `-<kebab>` concern / gate label |
| Relation to gate pad | Timestamp width is **orthogonal** to `gNNN` / `fsNNN` padding — do not “correct” gate digits inside historical evidence kebabs unless a coordinated Migration rewrites citations |

#### 2b.7 Consistency checklist (numeric)

When adding or renaming a numbered slot, **all** of the following that exist **MUST** share the **identical** digit spelling **and** contextual id (same `gNNN-<kebab-context>` string):

1. `verify/gates/<id>/` basename
2. Compose service name
3. `verify/scripts/run-….sh` stem
4. `verify/matrices/matrix-….md` stem (when dedicated)
5. Evidence kebab substrings that cite the **live** gate id (new evidence only — do not rewrite history casually)
6. Matrix / README prose path citations for that slot

Folder orthography remains §1; filenames §2a; this subsection freezes **how numbers are written** and ties them to contextual slugs.

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
| Directory consistency (this doc) | **this SoT** | folder + filename + **numeric** naming / out / SKIP / registration |

## Appendix A — Forbidden / legacy aliases and recommended migration (folders)

**Convention SoT first.** This appendix lists **folder / id** renames for a **future harness wave**. Spec-only commits **MUST NOT** mass-rename the tree unless a row is marked **MUST** and the change is tiny and coordinated. Sibling B-gate WIP **MUST NOT** be clobbered.

### Forbidden going forward (new work)

| Form | Why |
|------|-----|
| New top-level dirs under `verify/` outside §1a allow-list | Breaks grouped overlay |
| New jammed G-ids (`g11l3`-style) | Prefer `gNNN-<kebab-context>` |
| New unpadded / two-digit-only G dirs (`g1`, `g01`) | Prefer `gNNN-<kebab-context>` (§2b) |
| `snake_case` / camelCase verify folders | Orthography lock |
| Committed `gates/<id>/target/` or `verify/target-*` | Host/clean-room Cargo skew |
| Empty `gates/dod-*/` scaffolds “for symmetry” | Script-first DoD/residual until oracle needed |
| Flat `g*` / `matrix-*` at `verify/` root | [[sot:root/structural-conventions]] rule 23 |

### Legacy aliases (grandfathered)

| Current | Preferred | Status |
|---------|-----------|--------|
| `gates/g11l3/` + compose `g11l3` + `run-g11l3.sh` | `g11-l3` everywhere (dir, compose, script, matrix rows) | **Migrated** hyphen unjam — live tree uses `g11-l3`; further `g011-…` pad/context still backlog; Claimed evidence blobs may still cite `g11l3` |
| Bare unpadded `gates/gN/` and unpadded variants (`g8-live-scale`, …) | `gNNN-<kebab-context>/` | **Grandfathered** — Appendix C / D |
| Repo-root `target-cleanroom/`, `target-pilot/` | Host advisory only; clean-room uses compose volume | **Keep** as host conveniences; do not promote into `verify/` |
| Gate-local `g41/seed.sh` | Acceptable peer helper | **Keep** unless folded into `scripts/` with compose update |

### Recommended renames (non-MUST — Migration backlog)

| From | To | Notes |
|------|----|-------|
| `verify/gates/g11l3/` | `verify/gates/g11-l3/` | **Done** — compose / `run-g11-l3.sh` / `matrix-l3.md` / matrix drivers updated; historical evidence blobs **not** rewritten |
| Bare / unpadded `gates/gN/` (+ matching compose / `run-*.sh` / matrices) | `gates/gNNN-<kebab-context>/` | **Backlog** — coordinate with verify-migration siblings; do not mass-rename in Spec-only waves; see Appendix C / D |
| Unpadded variants `g8-live-scale`, `g8-local-forge`, `g28-offline-clones`, `g11-l3` | `g008-live-scale`, `g008-local-forge`, `g028-offline-clones`, `g011-l3` (or richer context slug) | **Backlog** — already hyphenated; only pad (+ optional richer slug) |
| `fs*-advisory` | *(no change)* | Already three-digit compliant |

**Survey snapshot (folders, tip ≥ `9b27ea5`):** top-level = `scripts/` `matrices/` `gates/` `fixtures/` `out/` only; gate nested children observed = `out/` `src/` `tests/` `httpd/` (+ Cargo files); no `dod-*` dirs under `verify/`; host `target*` at repo root only (plus ephemeral `gates/g1/target` spill). Live G ids remain unpadded until a dedicated Migration wave.

## Appendix B — Filename skew survey and recommended migration

**Recommend only** unless a row is marked trivial **MUST**. Do **not** rename in Spec-only waves; do **not** clobber dirty B-gate WIP scripts.

### Forbidden going forward (new filenames)

| Form | Why |
|------|-----|
| New `snake_case` / camelCase harness or matrix basenames under `verify/` (non-Cargo) | §2a.1 orthography |
| Second root compose filename (`compose.yml`, `docker-compose.yaml`) | §2a.5 single authority file |
| New jammed script ids (`run-g11l3.sh`-style) | Prefer `run-gNNN-<kebab-context>.sh` |
| New unpadded / two-digit-only `run-gN.sh` | Prefer `run-gNNN-<kebab-context>.sh` |
| New stable receipt names that bypass `receipt.json` / documented `<concern>-receipt.json` | Breaks operator fixed paths |
| Underscore Python / shell helpers as new compose judges | Prefer kebab; helpers must not sole-judge Verified |

### Legacy / skew filenames (grandfathered until Migration)

| Current | Preferred | Status |
|---------|-----------|--------|
| `verify/scripts/run-g11l3.sh` | `run-g11-l3.sh` | **Migrated** with Appendix A |
| `verify/scripts/g19_eval.py` | `g19-eval.py` (or fold into `run-g19.sh` only) | **Migrated** — `run-g19.sh` / `run-g19-eval-only.sh` updated |
| `verify/fixtures/openai_compat_fixture.py` | `openai-compat-fixture.py` | **Migrated on disk** — B4 WIP callers updated; fixture may remain untracked until B4 lands |
| `verify/matrices/matrix-l3.md` | `matrix-g11-l3.md` (after id unjam) **or** keep as L3 milestone alias with README note | **Advisory** — milestone “L3” token predates `g11-l3` |
| `verify/matrices/matrix-l8-e2e.md` / `run-l8-e2e.sh` | Keep L8 milestone names **or** document as milestone alias beside G-series | **Keep** unless milestone SoT retires L8 label |
| `verify/matrices/matrix-s6-symmetry.md` / `run-s6-symmetry.sh` | Prefer `symmetry` / DoD family names already in §2 | **Advisory** — S6 historical label |
| `verify/scripts/probe-*.sh` | Keep as non-`run-` helpers | **Keep** — not compose primary judges |
| `gates/g41/seed.sh` | Keep short colocated name | **Keep** — §1c / Appendix A |
| Ephemeral `out/**/receipt-<sha>.json`, `metrics-debug.json`, `receipt-fail1.json`, … | Prefer overwrite stable `receipt.json` / `metrics.json`; keep sha dumps local-only | **Keep pattern for debug**; do not promote to Claimed citations |

### Recommended renames (non-MUST — filename Migration backlog)

| From | To | Notes |
|------|----|-------|
| `scripts/g19_eval.py` | `scripts/g19-eval.py` | **Done** — `run-g19.sh` / eval-only argv updated |
| `fixtures/openai_compat_fixture.py` | `fixtures/openai-compat-fixture.py` | **Done on disk** — coordinated with dirty B4 caller; do not invent Claimed |
| `scripts/run-g11l3.sh` | `scripts/run-g11-l3.sh` | **Done** with Appendix A |
| Stable multi-artifact names already compliant | *(none)* | `offline-clones-receipt.json`, living `*-matrix.csv`, root `docker-compose.yml` / `images.lock.json` / `README.md` match §2a |

**Survey snapshot (filenames, tip ≥ `4a99f8e`; Migration post-tip):** root files = `Dockerfile` `docker-compose.yml` `images.lock.json` `README.md` `.gitattributes`; scripts dominated by `run-*.sh` + few `*.ps1` / `g19-eval.py` / `probe-*.sh`; matrices = `matrix-*.md` + three `*-matrix.csv`; primary receipts = `receipt.json` / `metrics.json` / `summary.txt` with local sha/debug variants under gate `out/`; compose filename singular.

## Appendix C — Numeric notation skew and recommended migration

**Recommend only** unless a row is marked trivial **MUST**. Spec-only commits **MUST NOT** re-pad or mass-rename numbered ids; do **not** clobber dirty B-gate / verify-migration sibling WIP. If a sibling is mid-rename, land contextual/`gNNN` recommendations here without fighting their tree.

### Forbidden going forward (new numeric forms)

| Form | Why |
|------|-----|
| New **unpadded** G-series (`g1`, `g42`, `g100`) | §2b.3 — G-series is fixed-3 `gNNN` (supersedes 0.4.0) |
| New **two-digit-only** padded G-series (`g01`…`g99` as the width class) | §2b.2 / §2b.3 — use `g001`…`g099`, same class as `fsNNN` |
| Four-plus-digit G pads (`g0011`) without SoT amend | Range is `000`–`999` in this SoT |
| Unpadded or 2-digit FS ids (`fs7`, `fs07`) | §2b.5 — `fsNNN` is fixed-3 (**unchanged**) |
| New jammed digit+letter tokens (`g11l3`-style) | Prefer `gNNN-<kebab-context>` (Appendix A / D) |
| New **bare** `gNNN` without a concern slug | §1b — new gates MUST be `gNNN-<kebab-context>` |
| Declaring a new fixed-width series without amending this SoT | Width class must be explicit (§2b.2) |
| Mixing pad spellings for one slot across folder / compose / script / matrix | §2b.7 consistency |

### Legacy / skew (grandfathered)

| Current | Preferred | Status |
|---------|-----------|--------|
| Unpadded bare `g0`…`g41` (dirs, compose, `run-gN.sh`, many `matrix-gN.md`) | `gNNN-<kebab-context>` | **Grandfathered** — Migration backlog |
| Unpadded variants `g8-live-scale`, `g8-local-forge`, `g28-offline-clones`, `g11-l3` | `g008-live-scale`, `g008-local-forge`, `g028-offline-clones`, `g011-l3` (optional richer slug) | **Grandfathered** |
| `g11l3` (historical) | `g11-l3` then `g011-l3` / `g011-high-concurrency` | Hyphen **Migrated**; pad/context backlog |
| `matrix-l3.md` / L8 / S6 milestone labels | Keep alias **or** align after contextual Migration | **Advisory** — Appendix B |
| Evidence dirs citing live unpadded gate tokens (`…-g11`, `…-g19`) | Keep historical basenames | **Do not** rewrite solely for pad |
| `fs007` / `fs014` | Keep three-digit | **Compliant** |

### Recommended renames (numeric + context — Migration backlog)

Pad-only examples (still prefer adding a concern slug in the same wave when practical):

| From (live) | To (pad) | To (preferred contextual) |
|-------------|----------|-----------------------------|
| `g0` | `g000` | `g000-workspace-lib-tests` |
| `g1` | `g001` | `g001-loose-object-interop` |
| `g8` | `g008` | `g008-live-forge-smoke` |
| `g8-live-scale` | `g008-live-scale` | `g008-live-scale` (slug already present) |
| `g11` | `g011` | `g011-lease-product` |
| `g11-l3` | `g011-l3` | `g011-l3` or `g011-high-concurrency` |
| `g15` | `g015` | `g015-smart-http` |
| `g19` | `g019` | `g019-comparative-performance` |
| `g32` | `g032` | `g032-git-daemon-wire` |
| `g41` | `g041` | `g041-apache-dav-interop` |

**Align in the same Migration wave:** `verify/gates/<id>/`, compose service, `verify/scripts/run-<id>.sh`, and `verify/matrices/matrix-<id>.md` (plus helper peers `*-seed` / `*-dav` / volumes when owned by that gate). Cargo package names under gates MAY lag one step but **SHOULD** be updated in the same wave when touched.

**Survey snapshot (numeric, tip ≥ `9b27ea5`):** G-gates live unpadded `g0`–`g41` (gap at `g9`); variants `g8-*`, `g28-offline-clones`, `g11-l3`; FS advisory `fs007` + `fs014` already fixed-3; matrices mirror live unpadded `gN` / `mN` plus L3/L8/S6 aliases; evidence prefixes are 14-digit timestamps; **no** committed three-digit G-ids yet. **Padding + context migration required:** yes — backlog only in Spec-only waves (Appendix D for fuller slug list).

## Appendix D — Contextual rename proposals (survey)

**Non-MUST research list** for a future harness Migration. Sources: `verify/README.md` gate table, `verify/scripts/run-g*.sh` headers, `verify/matrices/matrix-g*.md` titles. Do **not** mass-apply while siblings hold dirty verify-migration trees — fold recommendations into their wave or land after.

| Live id | Proposed `gNNN-<kebab-context>` | Concern (short) |
|---------|----------------------------------|-----------------|
| `g0` | `g000-workspace-lib-tests` | Workspace library tests |
| `g1` | `g001-loose-object-interop` | Loose-object interop vs stock Git |
| `g2` | `g002-refs-index-roundtrip` | Refs + index round-trip |
| `g3` | `g003-index-commit-roundtrip` | Index-commit round-trip |
| `g4` | `g004-pack-idx-roundtrip` | Pack/idx local round-trip |
| `g5` | `g005-protocol-offline-fixtures` | Protocol offline fixtures |
| `g6` | `g006-cli-wave-l` | Wave L CLI argv |
| `g7` | `g007-smart-http-client` | Smart-HTTP remote client |
| `g8` | `g008-live-forge-smoke` | Live forge smoke |
| `g8-live-scale` | `g008-live-scale` | Live forge scale (slug kept) |
| `g8-local-forge` | `g008-local-forge` | Local forge (slug kept) |
| `g10` | `g010-store-facade` | Filesystem store facade |
| `g11` | `g011-lease-product` | Lease product (M10) |
| `g11-l3` | `g011-l3` | HighConcurrency L3 expansion |
| `g12` | `g012-porcelain-wave-pa` | Wave Pa porcelain |
| `g13` | `g013-porcelain-wave-pb` | Wave Pb porcelain |
| `g14` | `g014-cli-parity-production` | CLI-parity production (M9) |
| `g15` | `g015-smart-http` | Git server / smart HTTP (M11) |
| `g16` | `g016-journey-compare` | Journey-compare catalog |
| `g17` | `g017-concurrency-issue-compare` | Concurrency issue-compare |
| `g18` | `g018-concurrency-issue-compare` | Concurrency issue-compare (Cd) |
| `g19` | `g019-comparative-performance` | Comparative performance bars |
| `g20` | `g020-cli-symmetric-parity` | Full CLI symmetric parity |
| `g21` | `g021-cli-flag-symmetry` | Complete CLI flag symmetry |
| `g22` | `g022-celir-index-contention` | CELIR index contention |
| `g23` | `g023-celir-dual-path` | CELIR dual-path verified |
| `g24` | `g024-l3-celir-joint` | L3 + CELIR joint |
| `g25` | `g025-celir-index-scale` | CELIR index scale |
| `g26` | `g026-local-branch-pr-sim` | Local branch + PR sim |
| `g27` | `g027-smart-http-pr-contention` | Smart HTTP + PR contention |
| `g28` | `g028-live-private-arks` | Live private arks journeys |
| `g28-offline-clones` | `g028-offline-clones` | Offline clones (slug kept) |
| `g29` | `g029-server-dumb-http` | Server dumb HTTP |
| `g30` | `g030-server-ssh-command` | Server SSH command-mode |
| `g31` | `g031-server-protocol-v2` | Server protocol v2 |
| `g32` | `g032-git-daemon-wire` | `git://` daemon wire |
| `g33` | `g033-cred-askpass` | Credential-helper / askpass |
| `g34` | `g034-shared-index-contention` | Shared-index contention |
| `g35` | `g035-shallow-depth` | Client shallow `--depth` |
| `g36` | `g036-http-basic-auth` | Optional HTTP Basic |
| `g37` | `g037-gpgsig-ssh` | GPGSIG SSH create/verify |
| `g38` | `g038-gpgsig-openpgp` | GPGSIG OpenPGP / X.509 |
| `g39` | `g039-partial-clone-filter` | Partial-clone filter symmetry |
| `g40` | `g040-dumb-http-push-deletes` | Dumb HTTP push deletes |
| `g41` | `g041-apache-dav-interop` | Apache DAV dumb HTTP interop |

Research companion: [[research:root/20261111085000]] (optional short list / rationale).

## Why

`verify/` accumulated G-gates, symmetry DoD, release precheck, self-host scaffold, and Claimed residual slots under one tree. Without a single convention SoT, agents duplicate out paths, mix host and pin authority, invent folder, **file**, or **numeric** orthography, or register new residuals without Spec/residual cross-links. This document freezes the directory, folder-naming, filename-naming, **numeric-notation** (`gNNN` ≡ `fsNNN` width), and **contextual gate id** contract so Spec-first waves stay consistent, browsable, and fail-closed.

## Verification

1. `spec check` clean for this SoT and [[research:root/20261111082440]] when Spec CLI is available; otherwise peer review of links.
2. Cross-links from [[sot:root/verify-cleanroom-symmetry-matrix]], [[sot:root/arks-claimed-residual-gates]], [[sot:root/release-symmetry-precheck]], [[sot:root/production-residual-register]], [[sot:root/product-verification-clean-room]], [[sot:root/structural-conventions]], and `.spec/README.md`.
3. `verify/README.md` English map matches §1–§5 and folder/filename/numeric/context pointers (§1a–§1d / §2a–§2b / Appendix A–D). Live README may still show unpadded ids until Migration.
4. Clean-room pin check: container reports stock Git matching `verify/images.lock.json` (`git version 2.39.5`).
5. No product Claimed language added to public `docs/` from this design-only wave.
6. No mass rename of `gates/`, harness filenames, or **numeric pad / contextual id spellings** in Spec-only waves that amend §1b / §2b — Migration backlog (Appendix C–D) only unless a tiny MUST row is explicitly executed and coordinated with siblings.
