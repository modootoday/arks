---
id: research:root/20261111133600
title: In-scope stubs deepen — hub Partial×40 expand, CLI survey remainder, daemon Partial×2
status: active
created: 2026-11-11T04:36:00Z
updated: 2026-11-11T04:36:00Z
tags: [research, residual, hub, daemon, personality, stubs, no-invent-claimed, docker]
verify: manual
rel:
  depends_on:
    - research:root/20261111123556
    - sot:root/arks-in-scope-leftovers-wave
    - sot:root/arks-hub-cli-surface
    - sot:root/arks-cli-personality-factory
    - sot:root/arks-daemon-product-intent
  related:
    - verify/matrices/hub-gh-arks-e2e-matrix.csv
    - verify/matrices/cli-git-help-matrix.csv
    - verify/matrices/daemon-git-arks-e2e-matrix.csv
---

# In-scope stubs deepen — research note

English-only. Tip surveyed: **`5ae7ad9`**. Design ≠ Claimed until Docker receipts.

## Tip inventory leftovers after IW-ACTIONS/ASSETS/CHECKOUT/UX + survey expand + flags Parity

| Ledger | Residual |
|--------|----------|
| Hub v1 Partial | **39** CSV rows (SoT cites ×40); exit-2 stubs for unexpanded leaves |
| CLI survey Partial | **117** matrix rows; **12** SV-* Claimed; remainder honest Partial |
| Daemon Partial | **2** — `service:upload-archive`, `related:git:// client` |

## Hub IW-HUB-STUBS — honest Claimed subset (local forge)

| Leaf | Bar | Forge need |
|------|-----|------------|
| `pr status` | GET pull → print `number`/`state`/`title`/`draft` lines | none (GET exists) |
| `pr ready` | PATCH `draft:false` | `Pull.draft` + patch |
| `issue status` | GET issue → print status lines | none |
| `issue delete` | DELETE issue; list omits | DELETE + store |
| `repo edit` | PATCH description/private | PATCH repo |
| `repo set-default` | Write `default_repo` to client state; `repo_slug` fallback | client only |
| `api` deepen | `--method` + optional `--input` body; print `http_status=` | none |
| `status` | Aggregated stdout: `issues_open=` / `pulls_open=` counts | none |

**Stay Partial (no invent):** `pr diff|checks|review|lock|unlock|update-branch|revert`, `issue lock|unlock|pin|unpin|transfer`, `repo delete|fork|rename|archive|unarchive`, `auth refresh`, `label clone`, group-help-only rows.

## CLI survey remainder

Named expand **additional** peer exit-class rows (≥8 beyond prior 12 SV-*) under same gate `ARKS_PERSONALITY_SURVEY=1` or additive receipt schema. **Refuse** stampeding all 117 → Claimed.

## Daemon Partial×2

| Row | Disposition |
|-----|-------------|
| `service:upload-archive` | Enable+ustar via `git archive --remote=git://…` in flags harness → **Parity**; path-filter / non-ustar formats Stay Partial note |
| `related:git:// client` | **Stay Partial pointer** — do not double-count client lane |

## Cite

[[sot:root/arks-in-scope-leftovers-wave]] amend for IW-HUB-STUBS / survey remainder / upload-archive Parity bars.
