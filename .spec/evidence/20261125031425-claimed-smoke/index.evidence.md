---
id: evidence:root/20261125031425-claimed-smoke
title: Q21 serve ssh-smoke and agent smoke diagnostics
status: active
created: 2026-11-25T03:14:25Z
updated: 2026-11-25T03:14:25Z
tags: [q21, smoke, ssh, agent, docker, clean-room]
verify: docker
rel:
  depends_on:
    - sot:root/arks-claimed-residual-gates
---

# Evidence: Q21 smoke diagnostics

- **Bar:** [[sot:root/arks-claimed-residual-gates]] §Q21 (`arks serve ssh-smoke`, `arks agent provider smoke`, `arks agent review --smoke`)
- **Venue:** clean-room image `arks-verify-m1:0.1.0` (git 2.39.5, OpenSSH 9.2p1), script `verify/scripts/run-claimed-smoke-e2e.sh` (compose slot `dod-claimed-smoke`), run from a copy of branch tip `da68efb` (the copy has no `.git`, so `receipt.json` records `tip: unknown`)
- **Observed:** banner ok against a running `arks serve` SSH listener (rc 0); `--git-pack` reads the listener's KEXINIT (rc 0); closed port rc 1. `provider smoke` against the openai-compat fixture over `http://` answers `chat_http=200` (rc 0); `--mock` rc 0; closed port rc 1. `review --smoke` against the fixture and `--mock` print `review_smoke=ok` (rc 0); closed port rc 1. The fixture logged exactly 2 chat requests.
- **Not covered:** `https://` vendor endpoints (unsupported: `arks-agent` has no TLS client) and any live vendor or model quality. Does not prove B1/B2 or B4.
- **Artifacts:** `receipt.json`, `logs/`
