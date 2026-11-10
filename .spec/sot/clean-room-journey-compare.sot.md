---
id: sot:root/clean-room-journey-compare
title: Clean-room journey compare
status: active
created: 2026-11-08T18:24:57Z
updated: 2026-11-10T02:39:00Z
tags: [verification, docker, clean-room, journey, timing, perf, git-compat, observability, worktree, submodule, g17, g19]
verify: manual
rel:
  depends_on:
    - sot:root/product-verification-clean-room
    - sot:root/interop-test-matrix
    - sot:root/cli-surface
    - sot:root/cli-flag-parity-m9
    - sot:root/git-compat-surface
    - sot:root/public-surface
    - sot:root/program-roadmap
    - sot:root/storage-celir-boundary
    - sot:root/arks-remote
    - plan:root/20261109022956
    - adr:root/20261108083507
    - research:root/20261108073650
    - research:root/20261108071916
  evidence: [evidence:root/20261108183045, evidence:root/20261108200811, evidence:root/20261109065018, evidence:root/20261108233610, evidence:root/20261109114207, evidence:root/20261111001400, evidence:root/20261110003836, evidence:root/20261110110900, evidence:root/20261110111700, evidence:root/20261110113900]
---

# Clean-room journey compare

Version **0.4.0**. Normative intent for an **additive** Docker clean-room surface that runs diverse **user-journey** scenarios **side-by-side** (in-image stock Git vs `arks`, plus optional alias shim), recording **functional** outcomes and **timing/perf** metrics.

This SoT does **not** redefine [[sot:root/program-roadmap]] bars (**CLI-parity production = M9 / G14**; **lease = M10 / G11**; **production-server = M11 / G15**). It does **not** replace unlock gates **G0–G15** in [[sot:root/interop-test-matrix]]. It does **not** authorize public “faster than Git” marketing without a clean-room receipt and [[sot:root/public-surface]] honesty.

## Rule

### Purpose

1. Enable authoritative, reproducible **git vs arks** journey comparison inside the pinned clean-room (`verify/`, `arks-verify-m1` + in-image stock Git per [[sot:root/product-verification-clean-room]]).
2. For each catalog journey, record:
   - **Functional class** — exit codes / success-vs-refuse; same-`.git` / OID / ref / worktree parity **where claimed**.
   - **Timing/perf metrics** — wall-clock per step and per journey (schema below), always emitted when the harness runs.
3. Host-only timings are **advisory**. Authoritative comparison receipts MUST come from a fresh `compose run --rm g16` (or equivalent deterministic entrypoint).

### Relationship (additive)

| Surface | Role |
|---------|------|
| **G0–G15** | Unlock / production / regression oracles (pass/fail). Unchanged. |
| **G16** `journey-compare` | Observability + comparison evidence (func + timing). Additive. |
| **M9 / M10 / M11** | Production bars stay owned by roadmap + G14 / G11 / G15. G16 MUST NOT substitute. |

Journeys MAY map to claimed inventory rows from [[sot:root/cli-surface]] and existing matrices. They MUST NOT invent porcelain to “win” a row: refuse-closed / out-of-catalog inventory stays refuse / out-of-catalog.

### Dual-run protocol

4. **Identical seeds.** Each claimed journey starts from the same fixture seed script (files, env, argv shapes) for every peer lane.
5. **Lanes.**
   - **A — `git`:** in-image stock Git binary (pin from `verify/images.lock.json`).
   - **B — `arks`:** release-profile `arks` binary built inside clean-room (see Open/Closed decisions).
   - **C — alias (optional):** PATH argv shim `git`→`arks` where the catalog row requires alias twin ([[sot:root/cli-surface]] alias drop-in; `docs/alias-git-arks.md`).
6. **Isolation.** Each lane uses its own workdir (and temp root). No shared dirty `.git` across peers for timed steps.
7. **Determinism env.** Pin `GIT_AUTHOR_*` / `GIT_COMMITTER_*` (and dates) where Git semantics allow comparable commits. Do not invent commit-graph semantics beyond [[sot:root/git-compat-surface]].
8. **Peer oracle.** Stock Git remains the only compatibility peer. Zero `gix` / `gitoxide` ([[adr:root/20261108083507]]). CELIR excluded from product deps / Git-compat journeys ([[sot:root/storage-celir-boundary]]).
9. **Refuse rows.** For refuse-closed journeys, functional pass = `arks` (and alias, if run) exits non-zero **without** mutating the repo. Stock Git MAY implement the command; that is **not** a functional mismatch for refuse rows — record peer lane as `peer_implements` / skipped for parity assert.

### Journey catalog (v0.2.0)

Diverse claimed muscle-memory + refuse-closed negatives. Primary man-page cites for claimed behavior: git(1) / matching `git-<cmd>(1)` for the clean-room Git pin — summarize only; do not paste manuals. Rows map to [[sot:root/cli-surface]] / [[sot:root/cli-flag-parity-m9]] inventory — do not invent porcelain to “win” a row.

#### Wave-1 (landed v0.1.0)

| ID | Steps (argv sketch) | Functional class | Claim tier | Prerequisites | Out-of-scope / notes |
|----|---------------------|------------------|------------|---------------|----------------------|
| **J-LOCAL-01** | `init` → write file → `add` → `commit -m` → `status` → `log` | success both lanes; arks tip readable by stock `git log`/`rev-parse` | claimed (Wave L) | G6/G14 inventory | Offline local only |
| **J-BRANCH-01** | seed commit → `branch topic` → `checkout topic` → `switch` back | success both lanes | claimed (Wave Pa) | G12/G14 | |
| **J-MERGE-FF-01** | divergent FF-able topic → `merge --ff-only topic` on main | success both lanes | claimed (FF/`--ff-only` only) | G12/G14 | Non-FF / `--no-ff` remain refuse — not this row |
| **J-REBASE-01** | linear feature onto main → `rebase main` | success both lanes | claimed (non-interactive) | G13/G14 | Interactive `-i` refuse — not this row |
| **J-BUNDLE-01** | seed repo → `bundle create` → `bundle verify` | success both lanes | claimed (Wave Pb/Pc) | G13/G14 | Stands in for packaged-object journey |
| **J-REPACK-01** | seed → `repack -ad` | success both lanes | claimed (M15 Fs-3) | G16 | Replaces retired **J-REFUSE-02**; lease-honest `maint:gc` |
| **J-CLONE-LOCAL-01** | local smart-HTTP peer (in-clean-room) → `clone` URL dest | success both lanes | claimed (Wave H HTTP) | G7/G13 | **Not** live GitHub (G8). Remains the HTTP-local peer row |
| **J-ALIAS-01** | same as J-LOCAL-01 under `git`→`arks` argv shim | success on alias lane | claimed alias twin | G14 alias-ready | Shim PATH, not shell `alias` inside cargo |
| **J-WORKTREE-01** | seed → `worktree add -b <branch> <path>` → `list` → `move` → `remove` (and `prune` stale); locked remove fails closed | success both lanes for claimed forms; fail-closed lock/contention | claimed residual ([[plan:root/20261109022956]]) | G14 + repo `commondir` | Replaces retired **J-REFUSE-01**. Cite [git-worktree(1)](https://git-scm.com/docs/git-worktree). `--porcelain` / `lock` porcelain remain refuse |

#### Wave-2 (v0.2.0 expansion)

| ID | Steps (argv sketch) | Functional class | Claim tier | Prerequisites | Out-of-scope / notes |
|----|---------------------|------------------|------------|---------------|----------------------|
| **J-TAG-01** | seed commit → `tag -a -m` → `tag` list | success both lanes; stock Git reads tag object on arks lane | claimed (Wave Pc) | G14 | Lightweight-only is insufficient for this row |
| **J-STASH-01** | dirty worktree → `stash push` → `stash list` → `stash pop` | success both lanes | claimed (Wave Pa stash) | G12/G14 | Other stash subcommands remain ledger-scoped |

| **J-CHERRY-01** | two-branch linear tip → `cherry-pick` single clean commit | success both lanes | claimed (Wave Pb) | G13/G14 | Conflicted / ranges refuse — not this row |
| **J-DIFF-01** | seed → edit → `diff` → `add` → `diff --cached` | success both lanes | claimed (Wave Pa) | G12/G14 | Binary/word-diff extras not required |
| **J-RM-MV-01** | seed files → `mv` tracked → `rm` tracked → `commit` | success both lanes | claimed (Wave Pa) | G12/G14 | |
| **J-ARCHIVE-01** | seed → `archive --format=tar -o` HEAD | success both lanes; tar non-empty | claimed (Wave Pc) | G14 | zip / remote archive refuse/out |
| **J-REFLOG-01** | two commits → `reflog` / `reflog show` | success both lanes | claimed (Wave Pc) | G14 | expire/delete/exists not this row |
| **J-REVERT-01** | tip commit → `revert HEAD` single clean | success both lanes | claimed (Wave Pb) | G13/G14 | Conflicted refuse — not this row; stock Git uses non-interactive editor env |

| **J-REFUSE-02** | ~~`repack -ad` refuse~~ | — | **retired** | — | Superseded by **J-REPACK-01** (M15 Fs-3) |
| **J-SUBMODULE-01** | local repo → `submodule add` → `status` → `deinit` → `init` → `update`; gitlink `160000` + `.gitmodules` | success both lanes | claimed residual ([[plan:root/20261109022956]]) | G14 | Replaces retired **J-REFUSE-03**. Cite [git-submodule(1)](https://git-scm.com/docs/git-submodule) / [gitmodules(5)](https://git-scm.com/docs/gitmodules). `--recursive` nested refuse. **FW-SUBMOD-HTTPS:** `http(s)://` submodule add claimed via `arks-remote` `clone_http` (same wire as clone); local-path remains G16/G19 default peer |

#### Wave-3 (v0.3.0 additive)

| ID | Steps (argv sketch) | Functional class | Claim tier | Prerequisites | Out-of-scope / notes |
|----|---------------------|------------------|------------|---------------|----------------------|
| **J-CLONE-PATH-01** | seed local bare (or worktree) → `clone` **path** dest **and** `clone` `file://` absolute dest | success both lanes for both argv shapes | claimed (local path / `file://` hardlink-or-copy) | G7/G13 + [[sot:root/arks-remote]] local subset | **Not** G8. arks claimed transport = same-host local **hardlink-or-copy** of object dir (git-clone local default; fall back to copy) + pack-aware checkout — **not** `--shared` alternates and **not** piped upload-pack. Cite git-clone(1) local / `file://` forms + [[research:root/20261108071916]] (file transport note). SSH/`git://` remain refuse |

Deferred / not catalog-required: live remote (G8), LFS, forge APIs, SSH/`git://` server, CELIR/lease HighConcurrency product claims, server transport expansion beyond G15 claims, absolute ms pass/fail bars, `--shared`/alternates local clone, piped `file://` upload-pack clone.

### Metrics schema (`arks-journey-metrics/v1`)

10. Harness output MUST be machine-readable JSON conforming to schema id **`arks-journey-metrics/v1`** (fields below). A short human summary MAY also print to stdout.
11. **Required fields**

| Field | Meaning |
|-------|---------|
| `schema` | Literal `arks-journey-metrics/v1` |
| `run.venue` | `clean-room` for authoritative; `host-advisory` otherwise |
| `run.image_tag` / `run.image_digest` | From compose / `verify/images.lock.json` when known |
| `run.stock_git_version` | `git --version` inside venue |
| `run.arks_revision` / `run.arks_profile` | Binary identity; profile `release` for G16 v1 |
| `run.iterations` | Integer ≥ 1 (authoritative G16 default **≥ 3**) |
| `run.warm_policy` | Documented string (v0.2: `cold-workdir-per-lane`) |
| `run.aggregation` | `raw` when iterations=1; **`median_p95`** when iterations≥3 (required for authoritative G16) |
| `journeys[].id` | Catalog id |
| `journeys[].lanes[].peer` | `git` \| `arks` \| `alias` |
| `journeys[].lanes[].functional` | `pass` \| `fail` \| `skipped` \| `peer_implements` |
| `journeys[].lanes[].exit_overall` | Process exit for last failing step or 0 (last successful iteration when aggregated) |
| `journeys[].lanes[].wall_ms_total` | Under `raw`: sum of step walls. Under `median_p95`: **median** of per-iteration totals |
| `journeys[].lanes[].wall_ms_p95_total` | Required when `aggregation=median_p95`: nearest-rank **p95** of per-iteration totals |
| `journeys[].lanes[].steps[]` | `{ name, wall_ms, exit }` ; under `median_p95` also `wall_ms_p95` per step name |

11a. **Multi-iteration aggregation (normative when iterations≥3)**
    - Each catalog journey is executed `iterations` times with a **fresh workdir per lane per iteration** (`cold-workdir-per-lane`).
    - Functional class MUST hold on **every** iteration (any functional mismatch fails G16).
    - Per lane, collect `wall_ms_total` samples and per-step `wall_ms` samples (aligned by step `name`).
    - **Median:** odd `n` → middle of sorted samples; even `n` → arithmetic mean of the two central samples (integer ms, trunc toward zero after mean).
    - **p95:** nearest-rank on sorted ascending samples: rank = ceil(0.95 × n), 1-indexed, clamped to [1, n].
    - Emitted `wall_ms_total` / step `wall_ms` are medians; `wall_ms_p95_total` / step `wall_ms_p95` are p95.
    - Harness MAY override iterations via `ARKS_JOURNEY_ITERATIONS` (advisory host loops); authoritative compose default is **3**.

12. **Optional RSS / CPU (Closed v0.3.0 — emit when cheap; omit when not):**
    - When the venue can sample the **child process** cheaply (clean-room Linux: `/proc/<pid>/status` `VmHWM` for peak RSS; `/proc/<pid>/stat` utime+stime ticks → ms), lanes MAY include:
      - `peak_rss_kb` — peak resident set size in KiB (median under `median_p95`)
      - `peak_rss_kb_p95` — nearest-rank p95 when `aggregation=median_p95`
      - `cpu_ms` — user+system CPU time in ms for the timed child steps (median under `median_p95`)
      - `cpu_ms_p95` — nearest-rank p95 when aggregated
    - Steps MAY omit per-step RSS/CPU; lane-level rollup is enough for v0.3.
    - **MUST omit** the fields (do not invent `0`) when sampling is unavailable (e.g. host Windows advisory) or the peer step was not a spawnable child.
    - Optional metrics never fail G16 by themselves; functional class remains the hard gate. They MUST NOT authorize public “faster / leaner than Git” claims without [[sot:root/public-surface]] honesty + this receipt class.
13. **Honesty limits (normative)**
    - Container scheduling noise is real; do not claim micro-benchmark precision. Median/p95 reduce single-run noise; they do **not** authorize marketing superiority claims alone.
    - **Compile / link time is excluded** from product timings (build before timed section).
    - Debug vs release: G16 authoritative timings use **release** (`cargo build -p arks --release`). Other gates MAY keep debug.
    - Do not publish “faster than Git” on README/`docs/` without a clean-room evidence receipt and wording that matches [[sot:root/public-surface]]. Prefer summarizing under `.spec/evidence/`.
    - **No absolute ms pass/fail bars** in G16 that redefine M9/M10/M11. Aggregation is for honest recording/compare receipts only.
    - Absolute comparative ≥20% pass/fail is **M14 / G19 only**, staged **after** issue-resolution **G17/G18** (M12/M13) close — G16 MUST NOT authorize parallel perf-opt Done beside the issue track. Issue-resolution induced-fault compare is **G17/G18**, not G16.

### Pass criteria

14. **Functional** claims for catalog rows are **hard gates**: mismatch on claimed success/refuse class fails G16.
15. **Timing** is **recorded always** on successful harness progress; timing never silently dropped; authoritative runs use `median_p95` with iterations≥3.
16. **Perf regression absolute thresholds** remain **deferred** (must not invent pass/fail ms bars that redefine production). Aggregation method is **Closed** (see Open decisions).

### Evidence receipt format

17. Authoritative runs land Specfold evidence under `.spec/evidence/<utcCompact>-journey-compare-g16/` with `index.evidence.md` citing: this SoT + plan versions, image digest from lock (and local Id if distinct), stock Git version, arks revision/profile, journey functional table, linked `metrics.json` (`arks-journey-metrics/v1`), residuals/honesty (noise, excluded journeys, Open rows).
18. Evidence id convention: `evidence:root/<utcCompact>` matching directory prefix (same pattern as M9/M11 receipts).

### Clean-room entrypoint

19. Compose service **`g16`** → `verify/scripts/run-g16.sh` → build release `arks` → `cargo test --manifest-path verify/gates/g16/Cargo.toml` → write metrics artifact under `verify/gates/g16/out/` (gitignored raw host copies OK; evidence copies committed metrics snapshot).
20. Matrix: `verify/matrices/matrix-journey.md`. Reuse pinned image; bump `verify/images.lock.json` only if tooling requires it.

### Out of scope (this SoT)

- Implementing missing Git porcelain to pass a journey.
- CELIR-backed or lease product claims inside journeys.
- Raising/lowering M9/M10/M11 bars.
- Live GitHub G8 as a v1 requirement.
- Marketing Pages that assert perf superiority without receipts.

## Why

Functional oracles (G0–G15) already prove parity classes. Operators still need a **catalogued, timed, side-by-side** receipt surface so journey coverage and performance observations are reproducible in clean-room — without turning observability into a fake production milestone or dishonest marketing.

## Verification

1. `spec check` clean for this SoT, plan, and cross-linked SoTs.
2. `docker compose -f verify/docker-compose.yml run --rm g16` fresh container; metrics JSON validates `arks-journey-metrics/v1` with `aggregation=median_p95` and `iterations≥3`.
3. Catalog claimed rows (Wave-1 + Wave-2 + Wave-3) functionally green under git and arks (refuse rows: arks refuses); timings present with median + p95 fields; optional RSS/CPU present on clean-room Linux lanes when sampled.
4. Evidence receipt filed; host-only runs labeled advisory.
5. Zero `gix` / CELIR product edges on the harness path; M9/M10/M11 meanings unchanged.

## Open decisions (tracked until closed)

| Decision | Status | Notes |
|----------|--------|-------|
| Gate id / compose name | **Closed (v0.1.0)** | **G16** `journey-compare`; compose service `g16` |
| Binary profile for timings | **Closed (v0.1.0)** | **release** for G16; compile excluded from metrics |
| Warm / iteration policy | **Closed (v0.2.0)** | `cold-workdir-per-lane`; authoritative default `iterations=3`; aggregation `median_p95` |
| Perf metrics aggregation / SLO posture | **Closed (v0.2.0)** | Median + nearest-rank p95 recorded always on authoritative runs; **absolute ms pass/fail bars deferred** — must not redefine M9/M10/M11 |
| Absolute perf regression bars (ms / ≥20% fail gates) | **Closed for Pf-1 allow-list (M14/G19)** | G16 remains record-only for non-allow-list rows; **G19** enforces `arks_median ≤ 0.8× git_median` on `verify/matrices/matrix-g19.md` after M12+M13 claimed-set exit |
| Optional RSS/CPU metrics | **Closed (v0.3.0)** | Emit `peak_rss_kb` / `cpu_ms` (+ p95) when child sampling is available; **omit rather than invent** |
| Live-remote journeys (G8) | **Claimed (default-on when secrets)** | [[adr:root/20261110220000]]; live when token+URL; SKIP when absent; prefer offline/local peer for ladder |
| Path/`file://` clone lane | **Closed (v0.3.0; hardlink-prefer v0.4.0)** | **J-CLONE-PATH-01**; local path + same-host `file://` via hardlink-or-copy (not `--shared`; not piped upload-pack) |
| Public Pages perf summary | **Closed (honesty)** | Allowed only with receipt + public-surface honesty; default keep under `.spec/evidence/` |
| Pf5 aggressive ≤0.5× leftovers | **Claimed(partial 7/8) + Verified** | Claim amortization [[evidence:root/20261111001400]]; **tip remasure** `7a3d2dc` [[evidence:root/20261110113900]] (LOCAL **0.6703** `aggressive_na`; PATH **0.4615** ≤0.5 PASS; no 8/8); historical best-better-LOCAL `44dc566` [[evidence:root/20261110110900]] (LOCAL **0.6556** / PATH **0.4675**); prior tip `91fbf64` [[evidence:root/20261110111700]]; prior Verified `c1b0b24` [[evidence:root/20261110003836]] / `e466d56` [[evidence:root/20261110014600]]; prior 6/8 [[evidence:root/20261110234600]]; no silent keep-at-0.8 |
