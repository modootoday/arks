---
name: run-symmetry-matrix
description: >
  Use when running or interpreting clean-room three-pillar symmetry DoD
  (git↔arks, daemon, hub) via verify compose/scripts. Prefer before claiming
  Missing=0 or inventing PASS from host greens.
---

# Run symmetry matrix (three pillars)

## Preconditions

- SoT: `.spec/sot/verify-cleanroom-symmetry-matrix.sot.md`
- Rules `60-clean-room-verify`, `61-cross-os-pin-honesty`, `28-evidence-honesty`
- Pin image lock under `verify/` — do not invent digests

## Pillars

| Pillar | Compose (typical) | Entrypoint |
|--------|-------------------|------------|
| 1 CLI `git`↔`arks` | `dod-cli-matrix` | `verify/scripts/run-cli-git-arks-matrix-e2e.sh` / `run-symmetry-git.sh` |
| 2 Daemon | `dod-daemon-matrix` | `verify/scripts/run-daemon-git-arks-matrix-e2e.sh` / `run-symmetry-daemon.sh` |
| 3 Hub/`gh` | `dod-hub-matrix` | `verify/scripts/run-hub-gh-arks-matrix-e2e.sh` / `run-symmetry-hub.sh` |
| All | `dod-symmetry` | `verify/scripts/run-symmetry-dod.sh` |

## Steps

1. Read umbrella SoT tip currency table — note Partial vs Missing honesty.
2. Check sibling sole-runner Docker before heavy compose (rule `60`).
3. Run the needed pillar(s) via `docker compose -f verify/docker-compose.yml run --rm …`.
4. Record outcomes against living CSVs under `verify/matrices/` — Partial ≠ PASS.
5. Host/latest inventory skew is advisory relative to pin DoD (rule `61`).

## Must not

- Invent Claimed/PASS from SoT prose or host cargo greens.
- Collapse latest-oracle tracking into pin Missing=0 Claimed.
