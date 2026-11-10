# G22 bug-simulation notes

## Intent

Ultra-high concurrency against optional CELIR index dual-path (sidecar / rebuild / query) while FS `.git` + `arks-lease` remain authoritative for objects/refs.

## Induced faults

| ID | Induce | Expected |
|----|--------|----------|
| G22-P01 | N concurrent `rebuild_from_git` without `index:celir-index` lease | Fail-closed errors and/or single consistent winner; no torn Present as success |
| G22-P02 | Query during/after contended rebuild | Missing or correct effect only; never conflicting Present treated as OK |
| G22-R01 | N=32/128 readonly query after clean publish | All identical effects |
| G22-R02 | Shard writers + index readers | Tips verified; stock `git cat-file` OK |
| G22-X01 | Missing discrete-circuit mount | Gate FAIL (no stub) |

## Ops

- Sole runner lock under `verify/gates/g22/.run.lock.dir` (do not overlap G17/G19).
- Compose mounts `discrete-circuit` read-only beside `gir`.
