---
id: research:root/20261111135200
title: Stubs exhaust wave — metadata Claimed vs terminal Partial/Refuse
status: active
created: 2026-11-11T04:52:00Z
updated: 2026-11-11T04:52:00Z
tags: [research, residual, hub, daemon, personality, stubs, no-invent-claimed, docker]
verify: manual
rel:
  depends_on:
    - research:root/20261111133600
    - sot:root/arks-in-scope-leftovers-wave
  related:
    - verify/matrices/hub-gh-arks-e2e-matrix.csv
    - verify/matrices/cli-git-help-matrix.csv
    - verify/matrices/daemon-git-arks-e2e-matrix.csv
---

# Stubs exhaust wave — research note

English-only. Tip surveyed: **`442db4a`**. Design ≠ Claimed until Docker receipts.

## Remaining after IW-HUB-STUBS subset Claimed

| Ledger | Residual |
|--------|----------|
| Hub v1 Partial | **31** |
| CLI survey Partial | **117** (20 named SV Claimed; matrix not flipped) |
| Daemon Partial | **1** — `related:git:// client` pointer |

## Hub — Claimable without git/gh spawn (local forge metadata)

| Leaf | Bar |
|------|-----|
| `repo archive` / `unarchive` | PATCH `archived` true/false |
| `repo rename` | PATCH rename name; id preserved |
| `repo delete` | DELETE forge repo record (metadata only; never spawn git) |
| `issue lock` / `unlock` | PATCH `locked` |
| `issue pin` / `unpin` | PATCH `pinned` |
| `issue transfer` | PATCH move to another local owner/repo slug |
| `pr lock` / `unlock` | PATCH pull `locked` |
| `pr review` | POST review metadata (`event`/`body`); list on GET |
| `label clone` | Copy labels from `--from owner/name` into target repo |
| Group-help rows | Claimed discoverability: bare group → help exit 0 (already wired) |

## Hub — terminal Partial / Refuse (no invent)

| Leaf | Reason |
|------|--------|
| `auth refresh` | OAuth refresh — Refuse Claimed; Stay Partial cite |
| `pr diff` | Needs compare/git objects — terminal Partial |
| `pr checks` | CI check-runs — terminal Partial (no invent empty CI) |
| `pr update-branch` / `pr revert` | Git worktree / sequencer — terminal Partial |
| `repo fork` | Cross-repo git object copy — terminal Partial (metadata-only fork would invent) |

## CLI survey3

Batch peer exit-class rows (≥16 additional) under `ARKS_PERSONALITY_SURVEY=1` with Docker receipt `survey3_rows`. Matrix Partial×117 remainder stay Partial unless per-row evidence.

## Daemon pointer

`related:git:// client` — **not daemon flag DoD**; client lane Claimed elsewhere (R1 / flags clone oracle). Flip CSV to **Parity pointer** (same shape as `related:upload-pack CLI`) with cite — do not invent new client Claimed under daemon gate.

## Cite

Amend [[sot:root/arks-in-scope-leftovers-wave]] IW-HUB-STUBS / survey3 / daemon pointer.
