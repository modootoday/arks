---
id: evidence:root/20261110222100
title: G8 local-forge wire substitute (≠ GitHub live PASS)
status: active
created: 2026-11-10T22:21:00Z
updated: 2026-11-10T22:21:00Z
tags: [g008-live-forge-smoke, local-forge, wire, secrets-block, honesty, advisory]
rel:
  depends_on:
    - research:root/20261110222000
    - adr:root/20261110220000
    - adr:root/20261110221000
    - evidence:root/20261110210200
  evidence: []
files:
  - receipt.json
  - samples.json
---

# G8 local-forge wire (loopback) — ≠ GitHub live PASS

## Disposition

| Item | Status |
|------|--------|
| Compose | `g008-local-forge` |
| Receipt | `arks-g008-local-forge-wire/v1` `disposition=claimed-wire` |
| `github_live_pass` | **false** |
| Environment `g008-live-forge-smoke-live` secrets | Still **0** (unchanged hard block) |
| Product forge SLO | **Not claimed** |

Proves HTTPS git clone/push/fetch against loopback `arks serve-http` when GitHub secrets are absent. Must not be rewritten as R5 live remasure PASS.

Research: [[research:root/20261110222000]].
