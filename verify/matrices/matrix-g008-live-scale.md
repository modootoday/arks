# Matrix — G8+ live GitHub forge scale harness (Claimed)

**Gate:** additive to **G8**; not a numbered interop matrix row by default.  
**ADR:** [[adr:root/20261110221000]] (Claimed; supersedes [[adr:root/20261110161000]]); secrets policy [[adr:root/20261110220000]].  
**Honesty:** Functional live forge journeys are **G28** — not throughput, PR latency, or rate-limit capacity. **G26/G27** prove local/arks contention — not GitHub-class scale.

## Purpose

Operator-local measurement harness for throwaway GitHub-class HTTP timing. Product residual **FW-GH-LIVE-SCALE** is **Claimed** — real samples when secrets exist; honest no-measurement when absent. Never invent latency SLOs.

## Default (no secrets)

| Step | Command | Expected |
|------|---------|----------|
| SKIP receipt | `bash verify/scripts/run-g008-live-scale.sh` | `skip-no-measurement` receipt under `verify/gates/g008-live-scale/out/` |
| Policy | — | G7 remains default authoritative remote proof; harness Claimed ready |

## Default-on measurement (secrets present)

Set **both** of:

| Variable | Required | Notes |
|----------|----------|-------|
| `ARKS_G8_TOKEN` or `GITHUB_TOKEN` | non-empty | Throwaway PAT; scrub from logs |
| `ARKS_G8_REPO_URL` | HTTPS clone URL | Throwaway repo; delete after |
| `ARKS_G8_SCALE_SAMPLES` | optional | Positive int; default `5` in script |
| `ARKS_G8_SCALE_MEASURE` | optional | `0` force-off; unset/`1` = measure when secrets |
| `ARKS_G8_LIVE` | optional | unused for enablement; `0` is G8 force-off only |

```bash
# Example (operator shell — do not commit secrets)
export ARKS_G8_TOKEN=…
export ARKS_G8_REPO_URL=https://github.com/org/throwaway.git
bash verify/scripts/run-g008-live-scale.sh
```

Outputs: `verify/gates/g008-live-scale/out/receipt.json` + `samples.json` (durations only; no tokens).

## Docker (optional)

```bash
docker compose -f verify/docker-compose.yml run --rm \
  -e ARKS_G8_TOKEN -e ARKS_G8_REPO_URL \
  g008-live-scale
```

Service `g008-live-scale` shares the G8 clean-room image; sole-runner policy applies — do not collide with G14–G28 matrices.

## Forbidden

- Publishing medians as product SLO without clean-room G19-class governance.
- Claiming G28 PASS implies forge scale.
- Invented latency in docs or receipts.
