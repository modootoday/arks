---
id: evidence:root/20261110180000
title: FW-G9 Claimed --depth subset (G35 Docker PASS)
status: active
created: 2026-11-10T08:00:00Z
updated: 2026-11-10T23:30:00Z
tags: [fw-g9, g9, g35, shallow, depth, clean-room, claimed, verified]
rel:
  depends_on:
    - sot:root/arks-remote
    - sot:root/interop-test-matrix
    - sot:root/production-residual-register
    - sot:root/product-verification-clean-room
    - research:root/20261108071917
  evidence: []
files: []
---

# FW-G9 — Claimed `--depth` shallow subset (G35)

## Claimed subset (honest)

| In | Out |
|----|-----|
| `arks clone --depth N` / `arks fetch --depth N` on smart HTTP + SSH | `filter` / `blob:none` / `tree:0` / promisor |
| Request peer-advertised `shallow`; send `deepen N`; write `.git/shallow` | `deepen-since` / `deepen-not` / `deepen-relative` / `--unshallow` |
| Local path / `file://` `--depth` **refuse** | Invented capabilities |

Primary: [git-clone `--depth`](https://git-scm.com/docs/git-clone#Documentation/git-clone.txt---depthltdepthgt), [gitprotocol-capabilities](https://git-scm.com/docs/gitprotocol-capabilities) `shallow`, [gitprotocol-pack](https://git-scm.com/docs/gitprotocol-pack), [partial-clone](https://git-scm.com/docs/partial-clone) (non-claim).

Gate coordination: historical **G9** semantics; clean-room compose **G35** (G33=CRED, G34=SHARED-IDX).

## Host advisory (not product-verified alone)

| Check | Outcome |
|-------|---------|
| `cargo test -p arks-remote --lib` | PASS (incl. stock `git http-backend` `--depth 1`) |
| `cargo test --manifest-path verify/gates/g35/Cargo.toml` | PASS (G35-C01..C03 host) |

## Docker clean-room (product authority)

| Field | Value |
|-------|-------|
| Compose | `verify/docker-compose.yml` service `g35` |
| Entrypoint | `verify/scripts/run-g35.sh` |
| Matrix | `verify/matrices/matrix-g35.md` |
| Image | `arks-verify-m1:0.1.0` |
| Image Id | `sha256:b235193c9ae45991f057aec1551feb769bf796ca917af1745110917cf53dfada` |
| Stock Git | git version 2.39.5 |
| Tip at verify | `7d988f27be262d66f2d89bfae68f8b8e842c79fc` |
| Outcome | **PASS** — G35-C01..C03 (`g35_local_clone_depth_refuses`, `g35_clone_depth_1_writes_shallow`, `g35_fetch_depth_1_into_tracking_repo`) |

## Non-claims

- No `filter` / promisor / partial-clone product claim.
- No `deepen-since` / `deepen-not` / `deepen-relative` / `--unshallow`.
