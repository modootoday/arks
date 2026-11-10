---
id: adr:root/20261110161000
title: G8+ live GitHub forge scale measurement harness (opt-in)
status: superseded
created: 2026-11-10T16:10:00Z
updated: 2026-11-10T22:10:00Z
tags: [g8, live-forge, scale, harness, opt-in, residual, honesty, superseded]
rel:
  depends_on:
    - adr:root/20261110151000
    - sot:root/interop-test-matrix
    - sot:root/product-verification-clean-room
    - sot:root/production-residual-register
    - sot:root/g28-live-private-arks-journeys
    - plan:root/20261110150000
---

# G8+ live GitHub forge scale measurement harness (opt-in)

## Context

**G26** / **G27** / **G28** prove local or arks smart-HTTP contention and live-private **functional** journeys. They do **not** prove live GitHub-class **PR throughput**, **merge-queue latency**, or **REST/GraphQL rate-limit capacity**. Inventing latency numbers or marketing live forge scale without measurement is forbidden ([[sot:root/public-surface]]).

[[adr:root/20261110151000]] Refuse-closes default live forge smoke without operator secrets. Residual **FW-GH-LIVE-SCALE** MUST close as a **terminal harness + opt-in** path, not as bare Deferred.

## Decision

1. **Default product honesty — Closed (harness; no default claim):** Without opt-in, the product MUST NOT claim live GitHub PR/forge scale verification. Residual disposition is **Closed (measurement harness + opt-in)** — not Deferred.
2. **Harness surface:** Normative operator entry is `verify/matrices/matrix-g8-live-scale.md` and `verify/scripts/run-g8-live-scale.sh`. Default run writes a **dry-run / refuse-closed** receipt when scale measurement vars are absent (same secret policy as G8).
3. **Opt-in vars (all required for live measurement):**
   - `ARKS_G8_LIVE=1`
   - `ARKS_G8_TOKEN` or `GITHUB_TOKEN` (throwaway PAT; never commit)
   - `ARKS_G8_REPO_URL` (throwaway HTTPS peer)
   - `ARKS_G8_SCALE_MEASURE=1` (explicit scale harness arm)
   - Optional: `ARKS_G8_SCALE_SAMPLES` (positive integer; default documented in matrix)
4. **Allowed measurements:** Non-destructive timing of operator-selected Git HTTP operations against the throwaway repo (e.g. repeated `ls-remote`, small fetch/clone round-trips). MUST NOT create marketing SLOs or publish invented medians; results belong in operator-local receipts only unless a future evidence stub is authored.
5. **Forbidden:** Default-on live scale; claiming G28 functional PASS implies forge throughput; scrubbing tokens from logs is required on any live run ([[adr:root/20261110151000]] §3).
6. **Relation to G8:** Functional live smoke remains `run-g8.sh` / gate G8. Scale harness is additive documentation + script; it does not redefine M6/M9/M28 exits.

## Consequences

- [[sot:root/production-residual-register]] **FW-GH-LIVE-SCALE** → **Closed (harness + opt-in)** citing this ADR.
- Register section D live-scale row aligns; G28 remains functional-only Claimed.
- Public copy MUST continue to separate **functional journeys** from **forge scale/capacity**.

## Supersedes / amends

- Replaces residual **Deferred non-blocking** for live GitHub PR/forge scale.
- Does **not** amend G7 offline default or G28 journey receipts.

## Superseded by

- [[adr:root/20261110221000]] — FW-GH-LIVE-SCALE **Claimed**; measure when secrets present (aligned with [[adr:root/20261110220000]]).
