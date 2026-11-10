---
title: Forge scale harness
description: Maintainer secrets-gated G8 / live forge scale readiness — dry-run vs receipt-only live
nav_order: 86
sections:
  - model
  - dry-run
  - live
  - honesty
  - {c: cta, name: next}
---

# Forge scale harness

Maintainer notes for the **secrets-gated** live forge smoke (**G8**) and additive scale harness (**G8-live-scale**). End users do not need these paths — they are operator / evidence tooling. Functional private-forge journeys stay separate from throughput claims.

## Channel model

| Path | Entrypoint | When it runs |
|------|------------|--------------|
| G8 live smoke | `verify/scripts/run-g8.sh` (compose `g8`) | Default-on when token + throwaway repo URL are present; honest SKIP when absent |
| G8-live-scale | `verify/scripts/run-g8-live-scale.sh` (compose `g8-live-scale`) | Records operator-local timings when secrets present; honest no-measurement when absent |
| G8 local-forge | `verify/scripts/run-g8-local-forge.sh` (compose `g8-local-forge`) | Loopback `arks serve-http` clone/push/fetch wire when GitHub secrets are absent; **≠** GitHub live PASS |

Secrets live in Environment **`g8-live`** (or a local shell). YAML cannot invent tokens. Prefer a throwaway peer repo and scrub PATs from logs.

Force-off: `ARKS_G8_LIVE=0` (smoke), `ARKS_G8_SCALE_MEASURE=0` (scale). Offline remote proof remains **G7**. Local-forge proves the HTTPS git **wire** on a loopback peer only — never rewrite residual R5 as GitHub-hosted remasure PASS from local receipts.

## Dry-run (no live PASS)

From the Actions UI, run the **g8-forge-scale** workflow with **dry_run** enabled (`workflow_dispatch`). That path:

1. Prints a structural readiness report (`scripts/forge-scale-readiness.sh`).
2. Exercises both harness entrypoints **without** Environment secrets → SKIP / no-measurement receipts.
3. Does **not** call a live forge and does **not** invent latency numbers.

A green dry-run means **ops wiring is ready**, not that G8 live remasure or a product scale SLO passed.

Local structural check (no secrets):

```bash
bash scripts/forge-scale-readiness.sh
```

## Live samples (receipt-only)

1. Configure Environment `g8-live` with `ARKS_G8_TOKEN` (or usable `GITHUB_TOKEN`) and `ARKS_G8_REPO_URL`.
2. Dispatch **g8-forge-scale** with `dry_run=false`, or run the compose services locally with the same env vars.
3. Keep sole-runner honesty — do not collide with clean-room G14–G28 matrices or restart sole G19 remasure without honesty breakage.
4. Attach real receipts under `.spec/evidence/` only. Empty SKIP runs are not live PASS evidence.

How a claim would be earned: **receipt-only**. Harness Claimed (ready) is already separate from any future live remasure or product SLO. Never publish medians as product SLOs without explicit gate governance.

## Symmetry S5 / optional O1 disposition

Expanded DoD **S5** closed as a **documented secrets-block** ([[evidence:root/20261110113300]]). Optional **O1** re-probed Environment `g8-live` (**secret count = 0**) and shipped `scripts/g8-live-operator-path.sh` fail-closed operator path ([[evidence:root/20261110210200]]): harness remains Claimed-ready; live remasure PASS requires operator secrets + real receipts. Never invent latency from dry-run / SKIP.

## Local-forge wire (secrets absent)

When Environment `g8-live` secrets are empty, maximize structural readiness and optionally run:

```bash
docker compose -f verify/docker-compose.yml run --rm g8-local-forge
# or: bash verify/scripts/run-g8-local-forge.sh
```

Receipt schema `arks-g8-local-forge-wire/v1` with `disposition=claimed-wire` and `github_live_pass=false`. Operator steps for true GitHub remasure remain: mint throwaway PAT + peer URL → Environment secrets → `dry_run=false` / compose `g8` + `g8-live-scale` → attach real receipts.

## Honesty

- Live forge throughput, Checks, and merge-queue behavior are **not** product SLOs — see [Limits](../limits/).
- Dry-run / readiness success is **not** a G8 live PASS and **not** a scale latency PASS.
- Local-forge `claimed-wire` is **not** a GitHub-hosted G8 live PASS.
- G28 functional Claimed (when cited elsewhere) does **not** imply forge scale capacity.
- Tokens stay in env / Environment secrets only — never in the tree.

## Next

[Release channels](../release/) [Contribute](../contribute/) [Limits](../limits/)
