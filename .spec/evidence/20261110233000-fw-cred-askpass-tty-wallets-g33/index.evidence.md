---
id: evidence:root/20261110233000
title: FW-CRED-ASKPASS G33 TTY + OS wallets Claimed
status: active
created: 2026-11-10T23:30:00Z
updated: 2026-11-10T23:30:00Z
tags: [fw-cred-askpass, g33, credential, askpass, tty, wincred, libsecret, osxkeychain, clean-room, verified, claimed]
verify: clean-room
rel:
  depends_on: [sot:root/credential-helper-askpass, sot:root/interop-test-matrix, evidence:root/20261110224000]
files:
  - {path: g33-run4.log, sha256: 85a1e343eb971a610cd05e0de97c5ff66ea27f088fd89ecf78e3ba39de10337c, bytes: 9888, store: repo}
---

# FW-CRED-ASKPASS — G33 TTY + OS wallets Claimed

Extends [[evidence:root/20261110224000]] (C01–C05) with TTY terminal prompt and first-party OS wallet helpers per [[sot:root/credential-helper-askpass]] v0.2.0.

| Field | Value |
|-------|-------|
| Branch | `agent/fw-cred-askpass` |
| Gate | **G33** |
| Matrix | `verify/matrices/matrix-g33.md` |
| Compose | `docker compose -f verify/docker-compose.yml run --rm --no-deps g33` |
| Receipt | `g33-run4.log` (C01–C07 ok; `G33: PASS`) |

## Results

| Gate | Command | Outcome |
|------|---------|---------|
| **G33** | `docker compose -f verify/docker-compose.yml run --rm --no-deps g33` | **PASS** — C01–C05 retained; **C06** TTY PTY clone; **C07** wincred store + libsecret fill + osxkeychain clone via `ARKS_CREDENTIAL_WALLET_DIR` |

## Non-claims

- Forge OAuth browser UX / host Apps auth (AUTH residual).
- Bundled Git Credential Manager binary.
