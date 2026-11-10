# G25 CELIR index scale matrix (M24 successor)

**Authoritative runs:** `verify/docker-compose.yml` service `g25` (`verify/scripts/run-g25.sh`).
Host runs are advisory only.

**Owning SoT:** `.spec/sot/celir-index-scale-clean-room.sot.md`
**Plan:** `.spec/plan/20261109204934-m24-g25-celir-index-scale.plan.md`
**Parent:** G22 [[evidence:root/20261109204428]] (N=32/128 historical)

**Honesty:** Scale extension only; sole-runner OOM policy; no fake green after 137.

## Topology

| Tier | N | Required |
|------|--:|----------|
| Historical G22 | 32 / 128 | Already verified |
| Scale-required | **256** | Clean-room required |
| Stretch | 384 | Attempt (PASS or hard-block) |
| Stretch | 512 | Attempt (PASS or hard-block) |

## Cases

| Case | Gate | How |
|------|------|-----|
| G25-C01 | g25 | N=256 concurrent open/query — identical effects |
| G25-C02 | g25 | N=384 stretch |
| G25-C03 | g25 | N=512 stretch |
| G25-C04 | g25 | Sole-runner + discrete-circuit; ObjectStore Refuse |

## Done for Verified

Docker `g25` PASS at N=256 + evidence; stretch tiers recorded honestly.
