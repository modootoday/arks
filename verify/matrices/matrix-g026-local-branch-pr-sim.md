# G26 local branch + synthetic PR sim matrix

**Authoritative runs:** `verify/docker-compose.yml` service `g026-local-branch-pr-sim` (`verify/scripts/run-g026-local-branch-pr-sim.sh`).
Host runs are advisory only.

**Owning SoT:** `.spec/sot/local-branch-pr-sim-clean-room.sot.md`
**Plan:** `.spec/plan/20261109205507-g026-local-branch-pr-sim-local-branch-pr-sim.plan.md`

**Honesty:** Local branch/PR-ref scale ≠ GitHub PR throughput. Live forge Deferred (G8+).

## Cases

| Case | Gate | How |
|------|------|-----|
| G26-C01 | g026-local-branch-pr-sim | ≥2048 branch heads; stock `for-each-ref` count matches arks |
| G26-C02 | g026-local-branch-pr-sim | ≥2048 `refs/pull/N/head` (+ merge subset); stock resolve OK |
| G26-C03 | g026-local-branch-pr-sim | ≥32 leased concurrent tip workers; tip integrity; stock-readable |
| G26-C04 | g026-local-branch-pr-sim | Contended lease / lock fail-closed; no lock skip |
| G26-C05 | g026-local-branch-pr-sim | Planted `*.lock` induce-compare (git pathology vs arks error) |
| G26-C06 | g026-local-branch-pr-sim | Sole-runner lock vs heavy G17/G19/G22–G25 |

## Done for Verified

Docker `g026-local-branch-pr-sim` PASS + evidence stamp with **actual N**; residual may say **local branch + synthetic PR-env verified**.
