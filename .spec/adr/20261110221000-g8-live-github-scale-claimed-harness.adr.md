---
id: adr:root/20261110221000
title: G8+ live GitHub forge scale harness Claimed (measure when secrets)
status: active
created: 2026-11-10T22:10:00Z
updated: 2026-11-10T22:10:00Z
tags: [g8, live-forge, scale, harness, claimed, residual, honesty]
rel:
  depends_on:
    - adr:root/20261110220000
    - sot:root/interop-test-matrix
    - sot:root/product-verification-clean-room
    - sot:root/production-residual-register
    - sot:root/g28-live-private-arks-journeys
    - plan:root/20261110150000
  supersedes: [adr:root/20261110161000]
  evidence: [evidence:root/20261110223000]
---

# G8+ live GitHub forge scale harness Claimed (measure when secrets)

## Context

[[adr:root/20261110161000]] closed residual **FW-GH-LIVE-SCALE** as **Closed (harness + opt-in)** with `ARKS_G8_LIVE=1` + `ARKS_G8_SCALE_MEASURE=1` + secrets required for any sample. [[adr:root/20261110220000]] Claimed G8 functional live as **default-on when secrets**. Scale measurement must align: real samples when secrets exist; honest no-measurement when absent — never invent latency SLOs.

## Decision

1. **Claimed (harness ready):** Residual **FW-GH-LIVE-SCALE** is **Claimed**. Normative entry remains `verify/matrix-g8-live-scale.md` + `verify/scripts/run-g8-live-scale.sh` + compose `g8-live-scale`.
2. **Measure when secrets present:** When token (`ARKS_G8_TOKEN` or `GITHUB_TOKEN`) **and** `ARKS_G8_REPO_URL` are present, the scale harness MUST record operator-local timing samples (e.g. repeated `ls-remote`) by default. `ARKS_G8_LIVE=1` is **not** required (aligned with [[adr:root/20261110220000]]).
3. **Honest no-measurement when secrets absent:** Without token or URL, write a **SKIP / refuse-closed / no-measurement** receipt and empty samples — MUST NOT invent medians or product SLOs.
4. **Explicit force-off:** `ARKS_G8_SCALE_MEASURE=0` MAY force no-measurement even when secrets are present. Unset or `1` does not block default-on measurement when secrets exist.
5. **Allowed measurements:** Non-destructive timing against the throwaway peer only. Results are operator-local / evidence receipts — not public product SLOs unless a future gate reopens that claim.
6. **Forbidden:** Invented latency in docs or receipts; claiming G28 functional PASS implies forge throughput; unsanitized tokens in logs.
7. **Relation to G8:** Functional live smoke remains `run-g8.sh` / gate G8. Scale harness is additive; it does not redefine M6/M9/M28 exits.

## Consequences

- [[sot:root/production-residual-register]] **FW-GH-LIVE-SCALE** → **Claimed** citing this ADR (+ measurement receipt when secrets, else harness-ready SKIP).
- Register section D live-scale row aligns; G28 remains functional-only Claimed.
- Public copy MUST continue to separate **functional journeys** from **forge scale/capacity**.

## Supersedes / amends

- **Supersedes** [[adr:root/20261110161000]] enablement (triple opt-in including `ARKS_G8_LIVE=1`) with secrets-default-on measurement + Claimed residual.
- Does **not** amend G7 offline default or G28 journey receipts.
