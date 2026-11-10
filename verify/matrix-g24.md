# G24 L3 HighConcurrency + CELIR index joint matrix

**Authoritative runs:** `verify/docker-compose.yml` service `g24` (`verify/scripts/run-g24.sh`).
Host runs are advisory only.

**Owning SoT:** `.spec/sot/l3-celir-index-joint-clean-room.sot.md`
**Plan:** `.spec/plan/20261109204935-m24-g24-l3-celir-index-joint.plan.md`

**Honesty:** Joint gate; G11-L3 stays CELIR-free; shared-index N-way OUT; lease ≠ skip locks; not CELIR ObjectStore.

## Topology

| Tier | N | Venue |
|------|--:|-------|
| Required | 32 | Clean-room |
| Preferred | 128 | Clean-room |

## Cases

| Case | Gate | How |
|------|------|-----|
| G24-C01 | g24 | WriterMode::HighConcurrency opt-in accepted |
| G24-C02 | g24 | N=32 HC shard apply + concurrent index readers; stock Git OK |
| G24-C03 | g24 | N=128 HC shard apply + index query integrity |
| G24-C04 | g24 | Competing `*.lock` → Pipeline; leases empty |
| G24-C05 | g24 | ObjectStore Refuse; sole-runner + discrete-circuit |

## Done for Verified

Docker `g24` PASS + evidence stamp; residual may say **L3 + CELIR index joint verified**.
