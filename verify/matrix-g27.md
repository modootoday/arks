# G27 arks smart HTTP + synthetic PR contention matrix

**Authoritative runs:** `verify/docker-compose.yml` service `g27` (`verify/scripts/run-g27.sh`).
Host runs are advisory only.

**Owning SoT:** `.spec/sot/arks-smart-http-pr-contention-clean-room.sot.md`
**Plan:** `.spec/plan/20261109210427-g27-arks-smart-http-pr-contention.plan.md`

**Honesty:** arks smart-HTTP stand-in ≠ GitHub PR UX / Checks / merge-queue / live forge throughput. Live forge Deferred (G8+). SSH/`git://`/dumb/v2 Remain Refuse.

## Cases

| Case | Gate | How |
|------|------|-----|
| G27-C01 | g27 | ≥2048 branch + ≥2048 PR heads seeded; `git ls-remote` via arks smart HTTP |
| G27-C02 | g27 | ≥8 concurrent stock `git clone` vs arks smart HTTP |
| G27-C03 | g27 | ≥8 concurrent `git fetch`; no torn objects |
| G27-C04 | g27 | ≥8 concurrent distinct-ref pushes; server agrees |
| G27-C05 | g27 | Contended same-ref push fail-closed; tip resolvable |
| G27-C06 | g27 | Sole-runner lock vs heavy G17/G19/G22–G26 |

## Done for Verified

Docker `g27` PASS + evidence stamp with **actual N**; residual may say **arks smart-HTTP PR-contention verified**.
