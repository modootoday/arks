---
id: sot:root/product-finalization-and-release-readiness
title: Product finalization and release-readiness procedure (F1–F5)
status: active
created: 2026-11-28T04:30:00Z
updated: 2026-11-28T04:30:00Z
tags: [finalization, release-readiness, verification, clean-room, public-surface, agy, completion]
verify: manual
rel:
  depends_on:
    - sot:root/product-intent
    - sot:root/program-roadmap
    - sot:root/production-residual-register
    - sot:root/product-verification-clean-room
    - sot:root/verify-cleanroom-symmetry-matrix
    - sot:root/release-symmetry-precheck
    - sot:root/public-surface
    - sot:root/autonomous-operation
    - sot:root/agy-automated-development
    - research:root/20261128043000
  governs:
    - .spec/plan/20261128043000-product-finalization-f1-f5.plan.md
---

# Product finalization and release-readiness procedure (F1–F5)

Version **0.1.0**. Normative specification for closing the gap between **engine-complete
development** (`version = "0.0.0"`) and a **verified, release-ready `arks` binary**, executed
under [[sot:root/agy-automated-development]]. This SoT owns the **finalization design,
implementation sequence, and verification gates** (F1–F5). It does **not** alter Git wire/CLI
semantics (owned by domain SoTs) and does **not** authorize A2 release acts without the owner.

Gap baseline: [[research:root/20261128043000]].
Executable ladder: [[plan:root/20261128043000]].

## 1. Product goal lock (re-confirmed)

Finalization MUST preserve every core invariant of [[sot:root/product-intent]]:

1. **Single published binary:** `arks` only (`crates/arks`). Internal library crates (`arks-hub`,
   `arks-server`, `arks-agent`, etc.) are never published as separate product binaries.
2. **Standards first; zero `git`/`gh` subprocesses on product paths:** stock `git` (pin `2.39.5`)
   and `gh` remain clean-room oracles only.
3. **Same on-disk `.git` store:** filesystem `.git` is the sole Git-compat object store; CELIR
   remains withdrawn ([[adr:root/20261119200000]]).
4. **Three peer-symmetry pillars + adjacent self-host + explicit extension:**
   - Pillar 1 (`git` ↔ `arks` / personality `git`): pin-2.39.5 option inventory & flag-corpus
     symmetry + behavioral DoD.
   - Pillar 2 (`git-daemon` ↔ `arks daemon`): 37/37 `Parity` + 2 `arks-only`, Missing=0.
   - Pillar 3 (`gh` ↔ flat `arks pr`/`issue`/… + `arks hub`): 171/171 v1 `Claimed`, Missing=0,
     Deferred=0.
   - Adjacent hosting (`arks serve`): HTTP, SSH (B1/B2), TLS (B9), zero-downtime rebind (B7),
     forge API, push rulesets.
   - Extension (`arks agent`): B4 fixture e2e + diagnostics + HTTPS TLS client.
5. **Honest non-goals stay explicit:** SHA-1 collision immunity (Impossible; detect+refuse Claimed),
   live G8 latency SLO without secrets (owner-refused Q10), real mail-server TLS/CRAM-MD5
   (owner-refused Q9), trimmed over-design (`host-env`, in-process LFS, CELIR), and owner-gated L9
   publishing.

## 2. Finalization gates (F1–F5)

Every gate MUST close in order before the subsequent release claim is made.

### F1 — Code, test-isolation, and manifest hygiene (Level A0)

1. **Host-independent workspace tests:** `cargo test --workspace` MUST pass cleanly on machines
   whose `~/.gitconfig` configures external helpers (including Antigravity VMs with `gh` credential
   helpers). Specifically, `crates/arks/tests/credential_os_cli.rs` MUST isolate `HOME`,
   `GIT_CONFIG_GLOBAL`, and `GIT_CONFIG_NOSYSTEM` in its test harness rather than inheriting user
   config.
2. **Canonical Cargo metadata:** Root `Cargo.toml` MUST declare
   `repository = "https://github.com/modootoday/arks"` (removing the legacy `humanymous/gir` URL).
   Crate descriptions in `crates/arks/Cargo.toml` and `crates/arks-remote/Cargo.toml` MUST reflect
   their current production-aligned roles without historical `"Wave … (not production)"` stubs.
3. **Version & publish lock preserved:** `version = "0.0.0"` and `publish = false` MUST NOT be
   changed in F1 (remains Level A2 in F5).
4. **Exit proof:** `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
   `cargo test --workspace` (without manual `HOME` override), and
   `python3 -I verify/scripts/agy-governance-check.py gate` all exit 0; all 7 required CI checks
   green on `main`.

### F2 — Authoritative clean-room Docker re-receipt (Level A0)

After F1 merges to `main`, run the **sole-runner** Docker clean-room suite (one Docker run at a
time per host per [[sot:root/product-verification-clean-room]]) on the post-F1 `main` tip:

1. **Argv inventory + full flag corpus** (`verify/scripts/run-argv-inventory.py` and
   `verify/scripts/run-cli-flag-corpus-compare.sh` inside the pinned `arks-verify-m1:0.1.0` image):
   - All 1,852/1,852 pin-2.39.5 options accepted.
   - All flag-corpus lines (including the PR `#76` `format-patch` additions) are `SAME` or carry a
     documented reason in `post-pin-lines.tsv`; **zero** unreasoned `DIFF` lines.
   - Timing-sensitive corpora (`family-a3-tools`) run alone, never concurrently with other corpora.
2. **Three-pillar pre-release symmetry gate** (`dod-symmetry-prerelease` with
   `ARKS_RELEASE_BLOCK_HUB_MISSING=1`):
   - CLI pin `Missing = 0`, Daemon pin `Missing = 0`, Hub v1 `Missing = 0`, untracked pin leaves = 0.
3. **Latest-inventory continuity check** (`ARKS_INVENTORY_MODE=latest bash verify/scripts/run-symmetry-latest-inventory.sh`):
   - Zero untracked upstream `git` or `gh` leaves (advisory deltas logged honestly; never labeled
     pin-DoD Claimed).
4. **Committed receipt:** Commit scrubbed artifacts (`receipt.json`, `venue-note.txt`, `summary.json`,
   corpus results, prerelease summary) under `.spec/evidence/<timestamp>-final-pre-release-receipt/`.
   If any corpus line or matrix row regresses, fix the root cause in a separate A0 PR before closing
   F2.

### F3 — Public-surface human documentation finalization (Level A0)

1. **Zero Specfold wiki-link leakage in public docs:** `README.md` and every file under `docs/**`
   MUST contain **zero** raw `[[sot:...]]`, `[[plan:...]]`, `[[adr:...]]`, `[[research:...]]`, or
   `[[evidence:...]]` tokens; replace them with standard Markdown links to GitHub blob URLs or
   plain-English human prose ([[sot:root/public-surface]]).
2. **Receipt-synced public honesty:** Update `README.md`, `docs/completion.md`, and stale pillar
   pages (`docs/daemon.md`, `docs/arks-hub.md`, `docs/self-host.md`, `docs/agent.md`) to cite the
   F2 pre-release receipt and accurately reflect Q17–Q23 + `#76`–`#77` status (171/171 hub v1
   Claimed, 37/37 daemon Parity, 128 Verified / 36 reasoned Partial CLI survey, CELIR withdrawn,
   live G8 / L9 / SHA-1 immunity explicitly Not Claimed / Impossible).
3. **`CHANGELOG.md`:** Add a concise English Keep-a-Changelog-style `CHANGELOG.md` at repo root
   with an `[Unreleased]` section summarizing the verified `arks` surface for the first release.
4. **Automated regression guard:** Extend `verify/scripts/agy-governance-check.py` (`gate`) to fail
   if any modified file in `README.md` or `docs/**` introduces raw Specfold `[[...]]` tokens.

### F4 — Release pipeline hardening and dry-run verification (Level A1 `needs-human`)

1. **Tighten hub gate:** Update `.github/workflows/release.yml` to pass `block_hub_missing: true`
   to `release-symmetry.yml` (enforcing `Missing = 0` across all three pillars now that hub v1
   `Missing = 0` is proven by F2).
2. **Dry-run execution on GitHub Actions (`main`):**
   - Trigger `release-symmetry.yml` (`block_hub_missing=true`) via `workflow_dispatch` and confirm
     both `pin-symmetry` and `latest-inventory` legs pass.
   - Trigger `release.yml` (`dry_run=true`) via `workflow_dispatch` and confirm `symmetry-precheck`,
     3-OS `dist` (`x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`, `aarch64-apple-darwin`),
     and `dry-run-report` pass with valid archives, `LICENSE*` files, and `SHA256SUMS`.
3. **Honesty boundary:** A `workflow_dispatch` dry-run MUST NOT create a GitHub Release or invoke
   any OIDC publish job, and MUST be labeled **release-pipeline dry-run verified (not live publish)**.
4. **Recorded measurement:** Record the workflow run URLs and artifact checksum summary in the A1 PR
   body / `.spec/evidence/<timestamp>-release-dry-run/`.

### F5 — First public release cut and maintenance-mode transition (Level A2 — Owner Only)

Agents prepare the exact checklist and commands, then **stop** for owner execution:

1. **Release cut PR (owner-approved):** Bump `[workspace.package] version` in `Cargo.toml` from
   `0.0.0` to the owner-chosen semver (recommended: `0.1.0`), promote `CHANGELOG.md` `[Unreleased]`
   to `[0.1.0] - <date>`, and decide whether `crates/arks/Cargo.toml` stays `publish = false`
   (GitHub Release binaries only) or flips to `publish = true` (requiring crates.io Trusted
   Publisher setup and publishable workspace path-crate dependency strategy).
2. **Tag & release:** Owner creates and pushes annotated tag `v0.1.0` on green `main` →
   `release.yml` runs `symmetry-precheck` → `dist` → drafts GitHub Release with `SHA256SUMS` →
   owner reviews and publishes the GitHub Release (and optionally approves GitHub Environment OIDC
   jobs `crates` / `npm` / `pypi` / `rubygems`).
3. **Maintenance-mode transition:** Once F1–F4 are merged and F5 is either cut or explicitly held
   under owner policy, mark [[plan:root/20261128043000]] `archived` (Done) and enter
   [[sot:root/autonomous-operation]] §11 maintenance mode.

## 3. Parallel execution blueprint under `agy`

To prevent hot-file contention and CI thrash during finalization ([[sot:root/agy-automated-development]]):

| Unit | Tier | Lane | Hot files touched | Dependency |
|------|------|------|-------------------|------------|
| **F1** | Tier A | `L-cli` (small manifest + test fix) | `Cargo.toml` | None (starts immediately on `main`) |
| **F2** | Tier A (sole Docker) | `L-verify` | None | After F1 merges (receipts the exact F1 tip) |
| **F3** | Tier A (subagents per doc group) | `L-gov` | `docs/completion.md` | After F2 merges (cites F2 receipt ID) |
| **F4** | Tier A | `L-gov` | None | After F2 merges (can run after F3 or disjoint with F3 if hot files do not overlap; sequential F3→F4 preferred) |
| **F5** | Owner A2 | `L-gov` | `Cargo.toml` | After F1–F4 merged and green |

## 4. Honesty and stop conditions

1. Design and dry-run greens NEVER invent `Claimed`, `Verified`, or live registry publish `PASS`.
2. Never run two Docker clean-room suites concurrently on one host during F2.
3. Never bump `version`, flip `publish = true`, or push `v*` tags without explicit owner direction
   (A2).
4. If F2 uncovers any unreasoned `DIFF` or untracked upstream leaf, stop F2 receipt promotion, fix
   the root cause in a clean A0 PR, and re-run F2 from the new `main` tip.

## 5. Verification

1. Every phase F1–F4 lands via an `agy/<lane>/<slug>` PR passing all 7 required checks +
   `agy hygiene (advisory)`.
2. F2 commits a scrubbed sole-runner Docker receipt under `.spec/evidence/` with 0 unreasoned DIFFs
   and 0 Missing across CLI, daemon, and hub pillars.
3. F3 leaves zero raw `[[...]]` Specfold tokens in `README.md` and `docs/**`, verified by
   `agy-governance-check.py gate`.
4. F4 records green GitHub Actions run IDs for `release-symmetry.yml` (`block_hub_missing=true`)
   and `release.yml` (`dry_run=true`).
5. F5 checklist is delivered to the owner in Korean with exact commands and options.
