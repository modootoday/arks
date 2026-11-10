# G22 CELIR index contention matrix (M24)

**Authoritative runs:** `verify/docker-compose.yml` service `g022-celir-index-contention` (`verify/scripts/run-g022-celir-index-contention.sh`).
Host `cargo test` with `--features celir-index` / `celir-index-stress` is advisory only.

**Owning SoT:** `.spec/sot/celir-index-contention-clean-room.sot.md`
**Plan:** `.spec/plan/20261109203949-m24-g022-celir-index-contention-celir-index-contention.plan.md`

**Honesty:** CELIR index-only; FS `.git` sole object store; leases ≠ skip Git locks; G11-L3 shared-index N-way OUT; not CELIR-backed Git verified.

## Topology

| Tier | N agents | Venue |
|------|----------:|-------|
| Smoke | 8 | Host advisory OK |
| Stress-tens | 32 | Clean-room required |
| Stress-hundreds | 128 | Clean-room required |

## Cases

| Case | Gate | How |
|------|------|-----|
| G22-C01 | g022-celir-index-contention | N=32 concurrent open/query after publish — identical effects |
| G22-C02 | g022-celir-index-contention | N=128 concurrent open/query after publish — identical effects |
| G22-C03 | g022-celir-index-contention | N=32 concurrent rebuild without index lease — fail-closed; leased rebuild restores integrity |
| G22-C04 | g022-celir-index-contention | N=32 lease shard Git writers + concurrent index readers; stock `git cat-file` OK |
| G22-C05 | g022-celir-index-contention | ObjectStore Refuse / default graph still CELIR-free; feature-on stress only |
| G22-C06 | g022-celir-index-contention | Sole-runner lock + discrete-circuit sibling mount present |

## Done for Verified

Docker `g022-celir-index-contention` PASS + evidence stamp; residual/roadmap may say index-contention **verified** for this gate only.
