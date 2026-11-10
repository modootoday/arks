---
id: adr:root/20261110210100
title: O1 G8 live — secrets hard block + fail-closed operator path
status: active
created: 2026-11-10T21:01:00Z
updated: 2026-11-10T21:01:00Z
tags: [o1, r5, g8, live-forge, secrets-block, operator-path]
rel:
  depends_on:
    - research:root/20261110205403
    - adr:root/20261110113300
    - adr:root/20261110220000
    - adr:root/20261110221000
    - sot:root/interop-test-matrix
    - sot:root/production-residual-register
  supersedes: []
  evidence: [evidence:root/20261110210200]
---

# O1 — Live G8 remasure: secrets hard block + operator path

## Context

Optional ladder **O1** / residual **R5** requires live G8 / G8-live-scale remasure when Environment secrets exist, else structural + documented hard block (**≠** invent PASS). Tip probe of Environment `g8-live` returns **secret count = 0**; process env has no `ARKS_G8_TOKEN` / `ARKS_G8_REPO_URL`. Prior S5 ([[adr:root/20261110113300]]) already recorded secrets-block; O1 pushes the operator fail-closed path as far as secrets allow.

## Decision

1. **Hard block** live remasure Claimed/PASS while Environment `g8-live` has zero secrets and process env lacks token+URL.
2. **Ship** `scripts/g8-live-operator-path.sh`: probe Environment secret *names* via `gh` when available; run structural readiness; exercise harness SKIP receipts; write `operator-disposition.json` with `live_pass:false`.
3. **Retain** harness Claimed-ready + dry-run workflow. Do **not** invent latency numbers or attach fake samples.
4. Reopen to live remasure Claimed only after real compose receipts land under `.spec/evidence/`.

## Consequences

- O1 Done as **secrets hard-block + operator path** (not live PASS).
- Public `docs/forge-scale.md` cites empty-Environment hard block honestly.
- Residual R5 stays harness Claimed; live remasure open until secrets+receipt.
