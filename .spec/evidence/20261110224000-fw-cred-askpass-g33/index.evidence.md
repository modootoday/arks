---
id: evidence:root/20261110224000
title: FW-CRED-ASKPASS G33 Claimed
status: active
created: 2026-11-10T22:40:00Z
updated: 2026-11-10T22:53:00Z
tags: [fw-cred-askpass, g33, credential, askpass, clean-room, verified, claimed]
rel:
  depends_on:
    - sot:root/credential-helper-askpass
    - sot:root/cli-surface
    - sot:root/arks-remote
    - sot:root/interop-test-matrix
    - sot:root/production-residual-register
---

# FW-CRED-ASKPASS — G33 Claimed

## Context

Minimal non-interactive HTTPS credential fill Claimed subset:
`arks credential fill|approve|reject`, `credential.helper` invoke, `GIT_ASKPASS` /
`core.askPass` / `SSH_ASKPASS`, Basic Authorization on smart-HTTP client.
Primary: [gitcredentials](https://git-scm.com/docs/gitcredentials),
[git-credential](https://git-scm.com/docs/git-credential).

## Environment

| Field | Value |
|-------|-------|
| Branch | `agent/fw-cred-askpass` |
| Compose | `verify/docker-compose.yml` service `g33` |
| Image | `arks-verify-m1:0.1.0` |
| Image Id | `sha256:b235193c9ae45991f057aec1551feb769bf796ca917af1745110917cf53dfada` |
| Stock Git | `git version 2.39.5` |
| Matrix | `verify/matrices/matrix-g33.md` |
| Script | `verify/scripts/run-g33.sh` |
| Cargo target | isolated volume `arks-cargo-target-g33` |

## Results

| Gate | Command | Outcome |
|------|---------|---------|
| Host unit | `cargo test -p arks-remote credential::` | **PASS** (advisory; 7) |
| **G33** | `docker compose -f verify/docker-compose.yml run --rm --no-deps g33` | **PASS** — C01 fill via helper; C02 clone + `credential.helper`; C03 clone + `GIT_ASKPASS`; C04 fail-closed without creds; C05 no hard-coded fixture secrets |

## Non-claims

- TTY password prompts, forge OAuth UX, first-party wincred/osxkeychain/GCM product path.
- Host green alone is not Claimed; this receipt is Docker clean-room authoritative.
