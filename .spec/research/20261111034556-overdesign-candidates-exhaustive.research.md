---
id: research:root/20261111034556
title: Over-design candidates exhaustive (clarified triad)
status: archival
created: 2026-11-10T18:45:56Z
updated: 2026-11-10T19:21:00Z
tags: [intent, symmetry, overdesign, daemon, lfs, forge, celir, inventory, exhaustive, archival]
verify: manual
rel:
  depends_on:
    - research:root/20261111034141
    - research:root/20261111033936
    - sot:root/product-intent
    - sot:root/cli-surface
    - sot:root/public-surface
    - sot:root/remote-host-compatibility-surface
    - sot:root/storage-celir-boundary
  superseded_by:
    - research:root/20261111035700
    - sot:root/product-intent
    - sot:root/arks-hub-product-intent
    - sot:root/arks-daemon-product-intent
  evidence: []
---

# Over-design candidates exhaustive (clarified triad)

**Archival.** Tip at write: **`c45f85e`**. C-list informed trim [[research:root/20261111035700]] and single-release shape (tip ≥ `9df7fb9`: one `arks` binary; `arks hub` / `arks daemon` via internal libs). Active Claimed honesty lives in tip SoTs — do not re-Claim trimmed rows from this note.

## Clarified product lens (owner Won)

In-scope **only**:

1. **Git-symmetric alias** — `git` ↔ `arks` parity on same `.git`
2. **Race-condition fixes** — concurrency / lease relief vs stock Git lock serialization
3. **Performance improvements** — measured receipts; not marketing slogans

### Explicit server symmetry (NOT trim)

Stock **Git server command** parity is **(1)** under Wave Sv / **M11** / remote-host surface — including start listeners. **Do not trim** as forge/LFS productization.

| Command | Stock peer | Disposition |
|---------|------------|-------------|
| `arks upload-pack` | `git-upload-pack` | **KEEP (A)** |
| `arks receive-pack` | `git-receive-pack` | **KEEP (A)** |
| `arks http-backend` | `git-http-backend` | **KEEP (A)** |
| `arks serve-http` | Convenience smart(+dumb) HTTP listener around http-backend semantics | **KEEP (A)** |
| `arks update-server-info` | `git-update-server-info` | **KEEP (A)** |
| **`arks daemon`** | **`git-daemon`** (`git://` TCP) | **KEEP (A) — in-intent; NOT Candidate trim** |
| `serve-http --basic-auth` / file / env (G36) | Transport HTTP Basic (gitprotocol-http); **not** forge OAuth | **KEEP (A)** |

**Clarify vs LFS:** `serve-http` **Git** paths stay KEEP. Bolted-on **`info/lfs` batch/locks server** is a separate **C** row (LFS server productization), not a reason to delete daemon / upload-pack / receive-pack / HTTP Git backend.

---

## Method (exhaustive at tip ≥ `c45f85e`)

1. Re-read prior inventory [[research:root/20261111034141]] + GitHub-coupling [[research:root/20261111033936]].
2. Enumerate every `match` arm in `crates/arks/src/main.rs` + every `crates/arks/src/cmd/*.rs` module.
3. Workspace members in root `Cargo.toml` (14 crates); optional `celir-index` path-deps into sibling `discrete-circuit`.
4. Public `docs/**` (+ `_config.yml`, Gemfile, nav/footer), `scripts/**`, `verify/scripts/**` (product-facing vs CI harness), `.github/workflows/*`.
5. Claimed / Claimed-closest / Done(ops) rows in `docs/git-vs-arks.md` + [[sot:root/public-surface]] that are **not** (1)(2)(3).
6. Mark each **C** row: **Candidate trim** | **Keep as ops/CI-only** | **Mis-labeled Claimed product** | (marketing hygiene only).

---

## A / B reminder (unchanged; daemon called out)

**A (symmetric):** everyday porcelain/plumbing inventory; same-`.git`; alias docs; interactive FS1/FS3; client remotes; **server duplex including `daemon`**; signing + credential helpers; replace / bundle-uri / proc-receive / worktree / submodule; GUI/scalar refuse+reason; clean-room G14–G21.

**B (intentional asymmetry serving intent):** `arks lease`; L3 / shared-index M26; `arks lock`; lease-honest maint; journey perf allow-lists; G17/G18 / G26/G27 contention harnesses; in-engine (no product-path `git` wrapper).

---

## Complete C-list (over-design / beyond clarified triad)

Disposition relative to clarified triad. **KEEP rows for daemon are above — not listed as C.**

| # | Item | Path / cite | Tip SoT / docs | Disposition | Prior inventory? |
|---|------|-------------|----------------|-------------|------------------|
| C1 | `arks forge` → host `gh` | `crates/arks/src/cmd/forge.rs` | Claimed-closest thin-wrap; full forge Non-goal | **Candidate trim** (binary surface) | Yes |
| C2 | `arks forge recipe` | same | G5 recipe deepen | **Candidate trim** | Yes |
| C3 | Full forge Checks/PR/Issues/OAuth product | residual FW-FORGE-B | Impossible / Non-goal | **Keep as ops/CI-only** (absent) | Yes |
| C4 | G8 / `g008-live-forge-smoke-live` live forge **latency SLO as product** | `docs/forge-scale.md`; `g008-live-forge-smoke-forge-scale.yml`; FW-GH-LIVE-SCALE | Harness Claimed; live PASS Blocked | **Keep as ops/CI-only**; **Mis-labeled Claimed product** if sold as end-user feature | Yes |
| C5 | Pages-facing **Forge scale** maintainer doc | `docs/forge-scale.md`; footer nav | Ops honesty page on Pages | **Keep as ops/CI-only** (do not elevate to Product nav) | **Missed emphasis** |
| C6 | G28 live-private journeys + harness `api.github.com` PR create | `verify/scripts/run-g028-live-private-arks.sh` | Functional Claimed ≠ Checks UX | **Keep as ops/CI-only** | Yes |
| C7 | G5 forge-recipe verify gate | `verify/scripts/run-g005-protocol-offline-fixtures.sh` | Harness for C1/C2 | **Keep as ops/CI-only** | **Missed** |
| C8 | OIDC `release.yml` + Environments **`crates`/`npm`/`pypi`/`rubygems` as product** | `.github/workflows/release.yml`; `docs/release.md`; public-surface L4 | Ops Done ≠ registry PASS; L9 owner | **Keep as ops/CI-only** | Partial (OIDC yes; **multi-registry npm/pypi/rubygems under-called**) |
| C9 | npm / PyPI / RubyGems **packaging trees as product** | absent `npm/` / PyPI tree; Gemfile is **docs** only | Skip-until in release.md | **Keep as ops/CI-only** (not shipped product) | **Missed** |
| C10 | Pages-facing **Release channels** maintainer doc | `docs/release.md`; footer | Maintainer ops | **Keep as ops/CI-only** | Partial |
| C11 | Host CI `ci.yml` / CODEOWNERS / SECURITY Advisories | `.github/` | Repo hygiene | **Keep as ops/CI-only** | Yes |
| C12 | Pages / Jekyll remote theme / `docs/Gemfile` hosting | `docs/_config.yml`, `docs/Gemfile` | public-surface human lane | **Keep as ops/CI-only** | Yes (theme); **Gemfile callout new** |
| C13 | `arks host-env` cloud/CfAPI/LSM detect | `cmd/host_env.rs`; FS-007/WIN-003/FS-014 | Claimed(advisory+refuse) | **Candidate trim** from default product story; **Mis-labeled Claimed product** if marketed as sync safety | Yes |
| C14 | `arks-hostfs` crate (advisory-only host detect) | `crates/arks-hostfs` | Supports C13 | **Candidate trim** with C13 (or ops-support if advisory CLI kept) | **Missed as crate row** |
| C15 | In-process `arks lfs` client (fetch/pull/push/**lock**/locks/unlock + smudge batch/basic) | `cmd/lfs.rs`; `arks-remote` LFS | Claimed locks/batch; cli-surface historically **deferred LFS** from default inventory | **Candidate trim** of **in-process LFS productization**; thin external `git-lfs` handoff may remain (1) | Partial (bundled with server) |
| C16 | `serve-http` **`info/lfs`** batch + GET/PUT + **locks** / verify | `arks-server` `lfs.rs`; git-vs-arks LFS row | Claimed locks; tus/S3 handoff | **Candidate trim** of **LFS server productization** — **not** trim of Git `serve-http` itself | Yes (wording sharpened) |
| C17 | CELIR index dual-path (`celir-index` feature) | `arks-store` feature; G22–G25 | ObjectStore Refuse; index opt-in OFF | **Candidate trim** / keep default-OFF; **Mis-labeled Claimed product** if “CELIR-backed Git” marketed | Yes |
| C18 | Sibling **discrete-circuit** path-deps + `scripts/ci-stub-discrete-circuit.sh` | `arks-store/Cargo.toml`; CI stub | Feature-gated compile | **Keep as ops/CI-only** (supports C17); not a workspace product crate | **Missed** |
| C19 | Embedded forge OAuth browser | AUTH / CRED | Refuse / Impossible | **Keep as ops/CI-only** (absent) | Yes |
| C20 | Absolute SHA-1 collision **immunity** marketing | OBJ-004 | Impossible; detect+refuse Claimed | Marketing hygiene (not excess code) | Yes |
| C21 | “Faster than Git” marketing | public-surface §22 | Forbidden without receipts | Marketing hygiene | Yes |
| C22 | Operator readiness scripts sold as product | `scripts/release-channel-readiness.sh`, `forge-scale-readiness.sh`, `g008-live-forge-smoke-live-operator-path.sh` | Maintainer | **Keep as ops/CI-only** | **Missed** |
| C23 | Host pilot / private-forge symmetry scripts | `verify/scripts/run-gh-private-pilot-symmetry.ps1` (+ untracked evidence) | Pilot harness | **Keep as ops/CI-only** | **Missed** (post-tip worktree noise) |
| C24 | `.agents/**` / Specfold process agents | `.agents/` | Agent routing — not end-user product | **Keep as ops/CI-only** (dev process) | **Missed** (process surface) |

### Note on tip Claimed vs clarified intent

Several **C** rows are **already Claimed** / Claimed-closest on tip (`host-env`, `forge`, LFS locks, CELIR when feature on, G28). That is tip SoT breadth, not triad membership. Owner may amend SoT + trim CLI, or widen triad. This note does **not** invent PASS.

---

## Completeness vs prior inventory (`20261111034141`)

| Verdict | Detail |
|---------|--------|
| Prior C-list | **Not fully exhaustive** — correct on the largest binary suspects (`forge`, `host-env`, LFS server/locks, CELIR, G8/G28, OIDC, Pages) and correctly placed **daemon in A**. |
| Gaps closed here | Multi-registry **npm/pypi/rubygems** productization; Pages **forge-scale** / **release** maintainer docs; **arks-hostfs** crate; **discrete-circuit** path-dep/CI stub; **in-process LFS client** split from server; G5 forge-recipe gate; operator `scripts/*`; process `.agents/**`; Gemfile/Pages packaging callout; pilot symmetry script. |
| False alarms checked | No extra experimental **workspace** crates beyond optional CELIR feature. `cmd/inspect.rs` / `replay.rs` are ordinary git porcelain modules (blame/fsck / cherry-pick-rebase-revert) — **not C**. No `package.json` / PyPI package tree shipped. GUI/scalar remain refuse+reason (**A** honesty). |
| Daemon | Prior inventory already had daemon under **A**. This addendum **re-confirms KEEP (A)** under owner ask — **not trim**. |

---

## Scan receipts (paths touched)

- CLI dispatch: `crates/arks/src/main.rs` (incl. `daemon`, `serve-http`, `lfs`, `forge`, `host-env`, GUI/scalar refuse).
- Cmd modules (37): archive…worktree; server porcelain in `cmd/server.rs` (`cmd_daemon` → `serve_daemon`).
- Crates: hash/oid/object/store/refs/index/repo/protocol/pack/remote/lease/server/hostfs/arks — all needed for A/B except **hostfs≈C13/C14** and **store celir-index feature≈C17**.
- Docs product vs ops: Product nav = overview/concurrent/getting-started/alias/workflows; Honesty = git-vs-arks/completion/performance/limits; Footer Project = **release** + **forge-scale** (ops).
- Workflows: `ci.yml`, `release.yml`, `g008-live-forge-smoke-forge-scale.yml` only.

---

## Bottom line

1. **`arks daemon` = KEEP (A)** — git-daemon command symmetry / M11 server role; **in-intent, not over-design trim**.
2. Prior inventory’s **main binary C suspects remain valid**; this note makes the **C-list complete** for tip `c45f85e` by adding registry/Pages/hostfs/discrete-circuit/LFS-client/process/ops-script rows previously thin or missing.
3. Largest **Candidate trim** still: **`forge`**, **`host-env`(+hostfs)**, **LFS client+server productization**, **CELIR feature**.
4. Largest **ops-only** pack: OIDC multi-registry, Pages maintainer docs, G8/G28/G5 harnesses, discrete-circuit stub, `.agents`.

## Verification

Manual exhaustive cross-read at tip `c45f85e`. No new evidence gates run.
