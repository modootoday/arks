---
id: research:root/20261111034141
title: Symmetry intent vs over-design inventory
status: archival
created: 2026-11-10T18:41:41Z
updated: 2026-11-10T19:21:00Z
tags: [intent, symmetry, alias, lease, performance, overdesign, forge, inventory, archival]
verify: manual
rel:
  depends_on:
    - sot:root/product-intent
    - sot:root/cli-surface
    - sot:root/public-surface
    - sot:root/production-residual-register
    - sot:root/lease-concurrency-intent
    - sot:root/remote-host-compatibility-surface
    - sot:root/storage-celir-boundary
    - research:root/20261111033936
  superseded_by:
    - research:root/20261111034556
    - research:root/20261111035700
    - sot:root/product-intent
  evidence: []
---

# Symmetry intent vs over-design inventory

**Archival.** Tip at write: **`953be57`**. Superseded for active honesty by exhaustive C-list + trim + single-`arks` release SoTs. Keep as historical inventory only — **no invent PASS/Claimed**.

## Clarified product lens (owner Won — supersedes GitHub-only framing)

In-scope **only**:

1. **Git-symmetric alias** — `git` ↔ `arks` parity on same `.git`
2. **Race-condition fixes** — concurrency / lease relief vs stock Git lock serialization
3. **Performance improvements** — measured receipts; not marketing slogans

Anything else is **suspect excess** relative to this lens, even when tip SoT currently labels it Claimed / Claimed-closest / Done (ops).

### Honest tension with tip SoT

Tip [[sot:root/product-intent]] still names a **broader** bar (native engine + server role **M11**, GitHub-class verify peer, CELIR index opt-in, public-surface forge thin-wrap). This inventory uses the **clarified triad** as the cut. Where tip SoT Claimed exceeds the triad, the row is **Mis-labeled Claimed product** (relative to clarified intent) or **Candidate trim** — not a denial that tip SoT text exists.

## Method

1. Read tip SoT: product-intent, cli-surface, public-surface, residual, lease-concurrency, remote-host, celir-boundary; public `docs/git-vs-arks.md`, `docs/limits.md`, `docs/completion.md`.
2. Inventory CLI entrypoints in `crates/arks/src/main.rs` + notable `cmd/*`.
3. Bucket **A** / **B** / **C**; for **C** assign **Keep as ops/CI-only** | **Candidate trim** | **Mis-labeled Claimed product**.
4. Append short GitHub-coupling subsection (from [[research:root/20261111033936]]).

---

## A — Symmetric (in-intent)

Git alias / parity surfaces that belong under (1).

| Item | Path / cite | Why in-intent |
|------|-------------|---------------|
| Everyday porcelain + plumbing inventory | `crates/arks/src/main.rs` commands; [[sot:root/cli-surface]] M9–M16 / G20–G21 | Muscle-memory `git`→`arks` |
| Same-`.git` interchange | [[sot:root/git-compat-surface]], product-intent | Alias requires stock-readable store |
| Alias drop-in docs / shim honesty | `docs/alias-git-arks.md`; Pf5 ALIAS journey | (1) core |
| Interactive subset FS1/FS3 | `add -p`/`-i`/`-e`, `rebase -i`, `stash -p` | Stock git interactive parity |
| Client remotes (smart HTTP/SSH/`git://`, shallow, claimed filters) | `arks-remote`; G7/G9/G35 | Ordinary git remote client |
| Server duplex mirroring stock git server cmds | `upload-pack`, `receive-pack`, `serve-http`, `http-backend`, `daemon`, `update-server-info` | Stock `git-upload-pack` / `git-daemon` / http-backend symmetry — **not** forge UX |
| Signing + credential helpers | G33/G37/G38; `credential*` | git man-page parity |
| `replace` / bundle-uri / proc-receive / worktree / submodule | FS2/FS5/FS8 rows in `docs/git-vs-arks.md` | git feature symmetry |
| GUI / scalar **refuse+reason** | FS6/FS7 | Honest N/A for non-CLI — not an embed |
| Clean-room journey gates G14–G21 / G16 | `verify/` | Prove (1); not product UX |

---

## B — Intentional asymmetry that still serves intent

Race / perf shapes git does not ship the same way — still (2)(3).

| Item | Path / cite | Serves |
|------|-------------|--------|
| Product leases (`arks lease`) | `arks-lease`; [[sot:root/lease-concurrency-intent]]; M10 | (2) agent coordination without inventing forge APIs |
| Opt-in L3 high-concurrency + shared-index M26 | ADR L3 / G34 | (2) N-way writers |
| `arks lock` stale `*.lock` inspect/clear | `crates/arks/src/cmd/` lock | (2) lock-storm hygiene |
| Lease-honest `gc` / `repack` / `maintenance` | main.rs help text | (2) maint under leases |
| Journey perf allow-list ≤0.8× (M17/G19) + aggressive ≤0.5× honesty | `docs/performance.md`; clean-room-journey-compare | (3) measured only |
| Concurrency scenario catalog G17/G18 | multi-agent scenarios SoT | (2) verify races |
| G26/G27 synthetic PR-head contention (local / arks smart-HTTP) | residual register | (2) **harness** for contention — not GitHub UX (keep naming honest) |
| In-engine (no product-path `git` wrapper) | product-intent; ADR native engine | Enables (2)(3) allocator / in-process concurrency |

---

## C — Over-design / beyond clarified triad (suspect excess)

Not (1)(2)(3). Disposition relative to clarified intent.

| Item | Path | Tip SoT / docs say | Disposition |
|------|------|--------------------|-------------|
| `arks forge` → host `gh` | `crates/arks/src/cmd/forge.rs` | Claimed-closest thin-wrap; full forge Non-goal (`docs/git-vs-arks.md`) | **Candidate trim** (binary surface); docs may keep “use `gh`” |
| `arks forge recipe` | same | G5 recipe deepen | **Candidate trim** |
| Full forge Checks/PR/Issues/OAuth product | residual **FW-FORGE-B** | Impossible / Non-goal | **Keep as ops/CI-only** (absent) — do not Claim |
| G8 / `g008-live-forge-smoke-live` live forge **latency SLO as product** | `docs/forge-scale.md`; `g008-live-forge-smoke-forge-scale.yml`; FW-GH-LIVE-SCALE | Harness Claimed; live PASS Blocked (secrets=0) | **Keep as ops/CI-only**; **Mis-labeled Claimed product** if sold as end-user feature |
| G28 live-private journeys + harness `api.github.com` PR create | `verify/scripts/run-g028-live-private-arks.sh` `J-PR-GH-01` | Functional Claimed ≠ Checks UX | **Keep as ops/CI-only** (verify peer); not CLI product |
| OIDC Environments + `release.yml` as **product** | `docs/release.md`; public-surface L4 | Ops Done ≠ registry PASS; L9 owner | **Keep as ops/CI-only** |
| Host CI `ci.yml` / CODEOWNERS / SECURITY Advisories | `.github/` | Repo hygiene | **Keep as ops/CI-only** |
| Pages / docs theme hosting | `docs/_config.yml` | public-surface human lane | **Keep as ops/CI-only** |
| `arks host-env` cloud/CfAPI/LSM detect | `crates/arks/src/cmd/host_env.rs`; FS-007/WIN-003/FS-014 | Claimed(advisory+refuse) | **Candidate trim** from default product story; **Mis-labeled Claimed product** if marketed as sync safety |
| In-process `arks lfs` + `serve-http` `info/lfs` locks beyond git-lfs companion parity | `cmd/lfs.rs`; git-vs-arks LFS row | Claimed locks/batch; tus/S3 handoff | **Candidate trim** of **server LFS productization**; client smudge for alias may stay thin (1) — cli-surface historically excluded LFS from default inventory |
| CELIR index dual-path (`celir-index` feature) | [[sot:root/storage-celir-boundary]] | ObjectStore Refuse; index opt-in OFF | **Candidate trim** / keep default-OFF; **Mis-labeled Claimed product** if “CELIR-backed Git” marketed |
| Embedded forge OAuth browser | AUTH / CRED | Refuse / Impossible | **Keep as ops/CI-only** (absent) |
| Absolute SHA-1 collision **immunity** marketing | OBJ-004 | Impossible; detect+refuse Claimed | Not excess code; forbid marketing Claimed |
| “Faster than Git” marketing | public-surface §22 | Forbidden without receipts | Not excess code; forbid slogan |

### Note on tip Claimed vs clarified intent

Several **C** rows are **already Claimed** on tip under FS/G ladders (`host-env` advisory, `arks forge` closest, LFS locks, CELIR G22–G25 when feature on, G28). That is **tip SoT breadth**, not proof they belong under the clarified triad. Owner may: (a) amend SoT to match triad + trim CLI, or (b) widen triad again. This note does **not** invent PASS either way.

---

## GitHub-coupling subsection (prior survey)

Full detail: [[research:root/20261111033936]]. Short table:

| Name | Where | Verdict vs design |
|------|-------|-------------------|
| `arks forge` / recipe | `cmd/forge.rs` | In-design thin-wrap (tip SoT); **Candidate trim** under clarified triad |
| No octocrab/Octokit in crates | Cargo graph | Absent — good |
| G8 / g008-live-forge-smoke-live / g008-live-forge-smoke-forge-scale | `verify/` + `.github/workflows/` | Ops/harness — not product SLO |
| G28 + `api.github.com` | `run-g028-live-private-arks.sh` | Harness-only |
| OIDC `release.yml` | `.github/workflows/release.yml` | Ops — not product |
| `ci.yml` / CODEOWNERS / Pages | `.github/`, `docs/` | CI-only / docs host |
| LFS `git-lfs.github.com/spec/v1` string | `arks-remote` LFS | Protocol cite — not forge API |
| **Beyond-design embed** | — | **None** found at prior survey tip |

---

## Bottom line

1. Under clarified triad, **core A+B is the product**: CLI symmetry + leases/race + measured perf.
2. **Largest suspect excess in the shipped binary:** `arks forge`, `arks host-env`, and **LFS server/locks productization** beyond companion/handoff; CELIR remains opt-in but is outside triad.
3. **Ops/CI** (OIDC release, G8 harness, G28, Pages) should stay labeled **ops**, never end-user Claimed product.
4. Tip SoT currently **over-claims breadth** relative to clarified triad — amend or trim; do not invent new PASS.

## Verification

Manual cross-read of paths above at tip `953be57`. No new evidence gates run.
