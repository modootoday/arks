---
id: research:root/20261110211804
title: Post O-ladder hard blocks — practical unblock plans (G8 live, L9, OOS)
status: superseded
created: 2026-11-10T21:18:04Z
updated: 2026-11-10T21:25:00Z
tags:
  [
    hard-blocks,
    g008-live-forge-smoke-live,
    r5,
    o1,
    l9,
    crates-io,
    docs-rs,
    pixel-tui,
    proc-receive,
    oos,
    secrets,
    owner-policy,
    post-o-ladder,
  ]
rel:
  depends_on:
    - research:root/20261110205403
    - research:root/20261110105436
    - research:root/20261110095142
    - research:root/20261110085731
    - sot:root/production-residual-register
    - sot:root/public-surface
    - sot:root/interop-test-matrix
    - sot:root/arks-server
    - sot:root/cli-flag-parity-complete
    - sot:root/product-naming
    - adr:root/20261110210100
    - adr:root/20261110210300
    - adr:root/20261110210400
    - adr:root/20261110220000
    - adr:root/20261110221000
    - evidence:root/20261110210200
---

# Post O-ladder hard blocks — practical unblock plans

> **Superseded (scheduling):** Hard-block 3 recommendation (“do not schedule pixel TUI / proc-receive / GUI/LFS”) is **overruled** by Full Symmetry DoD [[research:root/20261110212414]]. G8 secrets (hard-block 1) and L9 owner policy (hard-block 2) remain valid as **non-FS** items.

## Question

After optional ladder **O1–O_final Done** at tip ≥ `be54977` on canon `modootoday/arks`, what are the **practical unblock paths** for the remaining hard blocks that are **not** closed as product Claimed/PASS:

1. **Live G8 remasure** — Environment `g008-live-forge-smoke-live` secrets = 0 (readiness Done; no live PASS)
2. **L9** — crates.io / docs.rs publish timing (owner-closed policy)
3. **Pixel TUI** / **proc-receive** / other explicit **OOS** still deferred

Research only. Do **not** invent Claimed/PASS, fake secrets, or live latency. Do **not** treat dry-run / SKIP as PASS.

## Method

1. Re-read residual register R5 / O-ladder / public-surface L9 / §F P4–P5 OOS rows.
2. Cross-check public honesty pages (`docs/forge-scale.md`, `docs/limits.md`, `docs/git-vs-arks.md`, `docs/release.md`).
3. Confirm live Environment state via `gh` API (names only).
4. Classify each block as **temporarily operator-blocked**, **owner-policy closed**, **SoT-amend reopenable**, or **permanently out of product** (until a new primary SoT claims it).
5. Rank quickest honest next actions.

## Tip honesty baseline

| Item | Value |
|------|-------|
| Tip (research session) | `be54977` — *Claim Apache DAV G41 PASS and close O-ladder currency.* |
| O-ladder | **O1–O_final Done** ([[research:root/20261110205403]]) |
| Environment `g008-live-forge-smoke-live` | **Exists**; Environments also include `crates` / `npm` / `pypi` / `rubygems` / `github-pages` |
| `g008-live-forge-smoke-live` secret count (API, names only) | **0** (re-probed 2026-11-10; matches [[evidence:root/20261110210200]]) |
| Process env `ARKS_G8_TOKEN` + `ARKS_G8_REPO_URL` | Expected absent in agent land path |
| CLI crate `publish` | `publish = false` on `crates/arks` (and sibling crates) |
| L9 | **Owner-closed** — not a development Open |
| Pixel TUI | **N/A** after O3 scripted Claimed |
| `proc-receive` | **N/A (unclaimed)** — MUST NOT advertise |

---

## Hard block 1 — Live G8 remasure (R5 / O1)

### What is blocked and why

| Layer | State | SoT / docs |
|-------|-------|------------|
| Harness | **Claimed (ready)** — G8 default-on when secrets; G8-live-scale measure when secrets | [[adr:root/20261110220000]], [[adr:root/20261110221000]], [[sot:root/interop-test-matrix]] |
| Ops readiness | **Done** — Environment exists; `g008-live-forge-smoke-forge-scale.yml` dry-run; `scripts/forge-scale-readiness.sh`; `docs/forge-scale.md` | Ladder **L5** ops ([[research:root/20261110095142]]); O1 operator path ([[adr:root/20261110210100]]) |
| Live remasure / latency samples | **Hard-blocked** | Environment `g008-live-forge-smoke-live` **secret count = 0**; no process-env token+URL → harness SKIP / no-measurement only ([[evidence:root/20261110210200]]) |
| Product forge throughput SLO | **Not claimed** | [[sot:root/production-residual-register]] R5; `docs/limits.md` / `docs/forge-scale.md` honesty |

**Why:** YAML and agents cannot invent Environment secret *values*. Dry-run / structural OK / SKIP receipts are explicitly **≠** live PASS. Residual R5 reopen requires **secrets + real receipts** under `.spec/evidence/` — never invented medians.

### Prerequisites

| Prerequisite | Detail |
|--------------|--------|
| GitHub Environment | `g008-live-forge-smoke-live` on `modootoday/arks` (already present) |
| Environment secrets (names) | **`ARKS_G8_TOKEN`** (throwaway PAT with push/clone on throwaway repo) **and** **`ARKS_G8_REPO_URL`** (HTTPS clone URL of a throwaway peer) |
| Optional local path | Same names in process env; workflow prefers Environment secrets and clears job `GITHUB_TOKEN` as silent substitute |
| Force-off knobs | `ARKS_G8_LIVE=0` (smoke off); `ARKS_G8_SCALE_MEASURE=0` (scale off) |
| Workflow | `.github/workflows/g008-live-forge-smoke-forge-scale.yml` — `workflow_dispatch` with **`dry_run=false`** selects Environment `g008-live-forge-smoke-live` |
| Operator scripts | `scripts/g008-live-forge-smoke-live-operator-path.sh` (probe + fail-closed); compose services `g008-live-forge-smoke` / `g008-live-scale` |
| Specfold | Attach measured receipts under `.spec/evidence/` before any public “live remasure Claimed/PASS” wording; residual R5 update only with receipts |
| Honesty bounds | Sole-runner — do not collide with clean-room G14–G28 / sole G19 remasure; scrub PATs from logs; throwaway repo preferred |

### Unblock options

#### Path A — Quickest honest (recommended): Environment secrets + Actions live dispatch

1. Owner/operator creates a **throwaway** GitHub repo (or org sandbox) for clone/fetch/push smoke.
2. Mint a scoped PAT (or fine-grained token) with access to that throwaway only; store as Environment secret **`ARKS_G8_TOKEN`**.
3. Store throwaway HTTPS URL as Environment secret **`ARKS_G8_REPO_URL`**.
4. Confirm via names-only probe: `gh api repos/modootoday/arks/environments/g008-live-forge-smoke-live/secrets` → `total_count ≥ 2` with those names.
5. Dispatch **g008-live-forge-smoke-forge-scale** with **`dry_run=false`** (Actions UI or `gh workflow run`).
6. Inspect job summary receipts: expect non-SKIP dispositions when secrets resolve; if still SKIP, secrets are missing/misnamed — do not invent PASS.
7. Download/copy `receipt.json` (+ `samples.json` durations only) into a new `.spec/evidence/<ts>-g008-live-forge-smoke-live-remasure/` package; update residual R5 / comparison row only after evidence lands.
8. Optionally delete throwaway repo / rotate PAT after attachment.

#### Path B — Local compose remasure (same honesty)

1. Export `ARKS_G8_TOKEN` + `ARKS_G8_REPO_URL` in an operator shell (never commit).
2. `bash scripts/g008-live-forge-smoke-live-operator-path.sh` → expect `secrets-present-manual-live` (still `live_pass:false` until harness run).
3. `docker compose -f verify/docker-compose.yml run --rm -e ARKS_G8_TOKEN -e ARKS_G8_REPO_URL g008-live-forge-smoke` and `g008-live-scale` (or bare `verify/scripts/run-g008-live-forge-smoke*.sh`).
4. Attach real out/ receipts under `.spec/evidence/`; same residual update rules as Path A.

#### Path C — Remain secrets-blocked (valid steady state)

Keep O1 Done as **secrets hard-block**. Product DoD does **not** require live forge PASS. Public docs already say harness ready ≠ latency PASS. Acceptable indefinitely.

### Risks / honesty constraints

- Never publish dry-run green or SKIP as G8 live PASS or scale median.
- Never use production PATs / primary repo URLs if a throwaway is available.
- Live samples ≠ public product SLO without separate gate governance (`docs/forge-scale.md`, matrix-g008-live-scale Forbidden section).
- Docker sole-runner cost / contention — do not stampede with G14–G28 matrices.
- Agent sessions without secret write access **cannot** unblock this alone.

### Recommended next action (ranked)

1. **Owner adds `ARKS_G8_TOKEN` + `ARKS_G8_REPO_URL` to Environment `g008-live-forge-smoke-live`** (minutes; unblocks Path A).
2. Dispatch `g008-live-forge-smoke-forge-scale` `dry_run=false`; attach evidence if measured.
3. If secrets will not be provided, leave Path C — document currency only; no fake PASS.

**Classification:** **Temporarily operator-blocked** (not permanent OOS). Harness Claimed; live PASS is optional evidence reopen.

---

## Hard block 2 — L9 crates.io / docs.rs publish timing

### What is blocked and why

| Layer | State | SoT / docs |
|-------|-------|------------|
| Ladder L9 | **Closed (owner policy)** | [[research:root/20261110095142]], [[sot:root/public-surface]] public-flip table |
| Release-channel ops (L4) | **Done (ops dry-run)** — Environments `crates`/`npm`/`pypi`/`rubygems` + `release.yml` dry-run | [[research:root/20261110085731]], `docs/release.md` |
| crates.io / docs.rs live publish | **Not scheduled** by Specfold; timing is operator choice | public-surface: “not a residual Open”; name lock ≠ reservation ([[sot:root/product-naming]]) |
| Package metadata | CLI crate `arks` has **`publish = false`** | `crates/arks/Cargo.toml` |

**Why:** L9 was explicitly **excluded from product-completion DoD** and from S-/O-ladders as non-goals (“do not reopen L9”). Closing L9 as owner policy means agents must **not** treat missing crates.io publish as incomplete product work. Unblocking is an **owner decision**, then packaging/OIDC flips — not a Specfold residual Open.

### Prerequisites (only if owner opens L9)

| Prerequisite | Detail |
|--------------|--------|
| Owner decision | Explicit reopen: record in [[sot:root/public-surface]] (and preferably a short ADR) that L9 timing is now “publish now / on tag `vX.Y.Z`” |
| Crate publish flags | Flip `publish = false` → true (or selective crate set) on intended packages; keep verify-gate crates false |
| Trusted publisher | crates.io OIDC trusted publisher bound to `release.yml` + Environment **`crates`** (already in L4 design) |
| Environment `crates` | Required reviewers / tag policy `v*` per release research (reviewer noted historically) |
| Version / CHANGELOG | Semver tag `v*`; draft Release assets already in channel model |
| docs.rs | Follows crates.io publish automatically for published crates; English metadata; no secrets/evidence in package |
| Specfold | Amend public-surface disposition; residual register note that L9 opened by owner; evidence of dry-run then live publish receipt — dry-run ≠ publish PASS |
| Naming honesty | crates.io name `arks` free-at-lock is snapshot only — publish/reserve before relying on exclusivity |

### Unblock options

#### Path A — Owner opens L9 + first crates.io publish (full)

1. Owner records reopen (public-surface / ADR): target version, which crates, whether docs.rs is in-scope (usually automatic).
2. Flip `publish` on selected crates; ensure package README/metadata public-surface clean.
3. Confirm crates.io trusted publisher + Environment `crates` reviewers.
4. Run **release** workflow **dry_run** once more (structural ≠ PASS).
5. Push annotated tag `v*`; approve `publish-crates` Environment job.
6. Attach publish receipt / crates.io URL evidence; update docs/release honesty if needed.
7. Confirm docs.rs builds for published crate(s).

#### Path B — Reserve / yank-risk hygiene without full product publish

1. Owner may publish a minimal placeholder or reserve name early (product-naming warning).
2. Still requires owner timing decision; does not reopen L9 as a development Open by itself.
3. Prefer Path A once packaging is intentional — avoid confusing pre-1.0 consumers.

#### Path C — Keep L9 closed (default / recommended until owner speaks)

1. Leave `publish = false`; L4 dry-run remains the honest ops bar.
2. Install story stays source / GitHub Release assets (`docs/release.md`, getting-started).
3. Agents: do **not** implement “publish to crates.io” as unfinished ladder work.

### Risks / honesty constraints

- Dry-run success is **not** a Verified registry publish PASS.
- Premature publish with `publish = false` flipped without owner reopen violates Specfold non-goals.
- Name collision risk grows while unpublished — owner tradeoff, not agent debt.
- npm/PyPI/RubyGems remain packaging-gated skips; opening L9 for crates.io does not auto-claim those channels.
- Signing catalogs (winget/apt/Authenticode/notarization) stay **OUT** of current release scope.

### Recommended next action (ranked)

1. **Do nothing in-repo until owner explicitly opens L9** (quickest honest; matches closed policy).
2. If owner wants publish: Path A with public-surface amend first, then tag + Environment approve.
3. Optional owner-only name-reserve discussion (Path B) if exclusivity anxiety rises — still not agent ladder work.

**Classification:** **Owner-policy closed** (temporarily blocked on human decision). Not permanent OOS of the *product CLI* — publish is a distribution channel choice. **Do not reopen from agents** without owner text in SoT.

---

## Hard block 3 — Pixel TUI / proc-receive / other explicit OOS

### What is blocked and why

These are **not** secrets-blocked and **not** L9. They are explicit **N/A / out of product / catalog N/A** after honest ladder closes.

| Item | Disposition today | Why | Reopen shape |
|------|-------------------|-----|--------------|
| **Pixel / curses TUI** (`rebase -i` menu UI, `add -i` menu, TTY pixel flows) | **N/A** | O3 Claimed **scripted/non-TTY** subset only; full TUI remains out ([[adr:root/20261110210300]], `docs/limits.md`, `docs/git-vs-arks.md`) | New SoT + wire + evidence for a TUI product bar (large) |
| **`proc-receive`** capability / rewrite | **N/A (unclaimed)** | O4 Claimed push-options → hooks only; MUST NOT advertise `proc-receive` ([[adr:root/20261110210400]], [[sot:root/arks-server]]) | SoT amend + advertise + clean-room; still ≠ HOOK-008 race Claimed |
| **HOOK-008** proc-receive / push-options races | **N/A** (catalog) | §F **FW-CAT-HOOK-008** | Dedicated hostile harness SoT |
| **force-with-lease on dumb WebDAV** | **N/A** | O4 non-claim | Separate SoT if ever wanted |
| **Full Apache litmus / every DAV class** | **N/A** | G41 Claimed git-http-push subset only | Litmus SoT |
| **`add -e` hunk edit**, **`stash -p`**, mergetool/difftool **GUI** | **N/A** | O3 retain list | Per-row SoT amend |
| **FW-GUI** (gitk / git-gui) | **N/A (exclusion)** | Permanent product OOS unless GUI SoT | Dedicated GUI SoT |
| **FW-LFS** / **FW-SCALAR** | **N/A (exclusion)** | Permanent product OOS unless dedicated SoT | Dedicated SoT |
| **FW-FORGE-B** (Issues/PR/Checks host UX) | **N/A (out)** | Use `gh` etc.; arks is revision layer | Scope-(B) SoT adoption |
| **Agent frameworks / IDE products** | Out of product | `docs/limits.md` | Product-intent change |
| **Docker as end-user runtime** | Out of product | Native CLI is the product | Product-intent change |
| **Forge OAuth** | Refuse / N/A | CRED surface | Credential SoT reopen |
| **Other filter-specs** beyond `blob:none` / `tree:0` | Refuse / N/A | R3 remainder | Filter SoT expand |
| **R6 Git pin advance** | Process-closed | Pin **2.39.5** | Deliberate bump + G14-class re-prove |

### Prerequisites (only for SoT-amend reopenables)

| Target | Prerequisites |
|--------|----------------|
| Pixel TUI | Owner/product-intent: arks ships interactive TTY UI; SoT amend cli-surface + flag-parity; design for non-agent terminals; large clean-room story; **do not** Claim from scripted O3 alone |
| `proc-receive` | Amend [[sot:root/arks-server]] to Claim advertise+rewrite; implement; evidence; still separate from HOOK-008 |
| Narrow CLI rows (`-e`, `stash -p`, …) | Per-flag SoT amend + scripted/host tests; cheaper than full TUI |
| True OOS (GUI/LFS/scalar/forge-B) | New primary SoT + program roadmap — treat as **new product**, not residual unblock |

### Unblock options

#### Path A — Leave deferred (recommended default)

Document as permanent-or-indefinite N/A. O3/O4 already took the Specfold-honest maximum without inventing TUI/`proc-receive`. Public comparison already labels pixel TUI N/A and proc-receive out of claim.

#### Path B — Narrow SoT-amend reopen (if product pull exists)

1. Pick **one** narrow row (e.g. `add -e`, or `proc-receive` advertise without HOOK-008 Claimed).
2. Write research → ADR → SoT amend → wire → evidence.
3. Update residual register + `docs/git-vs-arks.md` / limits only after evidence.
4. Do **not** bundle pixel TUI + proc-receive + GUI into one “unblock OOS” epic.

#### Path C — True new-product SoT (GUI / LFS / forge host UX)

Only with explicit owner product-intent change. Out of post-O-ladder residual work.

### Risks / honesty constraints

- Inventing pixel TUI Claimed from scripted env vars violates O3 ADR.
- Advertising `proc-receive` without implementation is protocol fraud — refuse.
- Relitigating §F P4 exclusions as “hard blocks to clear for Done” reopens closed product scope.
- Docker cost for any new clean-room twin; halt on OOM/stampede.

### Recommended next action (ranked)

1. **Treat pixel TUI + proc-receive + FW-GUI/LFS/SCALAR/FORGE-B as out-of-claim steady state** — no unblock required for product Done.
2. If a user-facing gap hurts: Path B on the **smallest** SoTable slice (not full TUI).
3. True OOS product expansions only after owner primary-SoT direction.

**Classification:**

| Item | Permanence |
|------|------------|
| Pixel / curses TUI | **Indefinite N/A** (temporarily deferred only if owner later wants TUI product) — **not** required for Done |
| `proc-receive` / HOOK-008 | **Indefinite N/A** until SoT amend — protocol optional |
| FW-GUI / LFS / scalar / forge-B / agents / IDE | **Permanently out of product** under current intent |
| Narrow refused flags / extra filters | **Temporarily refused**; reopen-only via SoT |

---

## Permanently out of product vs temporarily blocked (summary)

| Block | Kind | Unblock owner |
|-------|------|---------------|
| G8 live remasure PASS | **Temporary** (operator secrets) | Owner/operator with Environment secret write |
| L9 crates.io / docs.rs timing | **Temporary** (owner policy) | Owner SoT reopen + publish ops |
| Pixel TUI | **Indefinite N/A** (product choice) | Owner + large SoT; else never |
| `proc-receive` | **Indefinite N/A** | SoT amend + wire; optional |
| GUI / LFS / scalar / forge host UX / agent frameworks | **Permanent OOS** (current intent) | New product SoT only |
| L4/L5 dry-run readiness | **Already Done** | N/A — do not confuse with live PASS |

## Cross-ladder honesty

- **O1 Done (secrets hard-block)** does not mean live PASS; it means the ladder goal closed honestly when secrets were absent.
- **L9 closed** does not mean “crates.io failed”; it means timing is not Specfold debt.
- **O3/O4 Done** does not unlock pixel TUI or `proc-receive`; they explicitly retained those as N/A.

## Recommended global order (quickest honest first)

1. **If live forge evidence is desired:** owner fills `g008-live-forge-smoke-live` secrets → Path A remasure → evidence attach (Hard block 1).
2. **Otherwise accept Path C** for G8 — product already Done without live PASS.
3. **Leave L9 closed** until owner publishes a reopen decision (Hard block 2).
4. **Do not schedule pixel TUI / proc-receive / GUI/LFS** as post-O-ladder blockers (Hard block 3).
5. Optional later: one narrow SoT-amend reopen if a concrete user need appears — never invent Claimed.

## Verification (this research)

1. Tip ≥ `be54977`; O-ladder research + residual R5/O rows cited.
2. `g008-live-forge-smoke-live` secret count re-probed **0** via GitHub API (names only).
3. Public docs (`forge-scale` / `limits` / `git-vs-arks` / `release`) and public-surface L9 disposition cited.
4. No Claimed/PASS invented; no secrets created; no implementation in this note.
