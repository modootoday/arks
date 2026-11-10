# G34 — shared-index contention (M26)

**Gate:** `shared-index-contention`  
**Milestone:** M26 / Phase Si  
**Unlock:** [[adr:root/20261110074000]]  
**SoT:** [[sot:root/shared-index-nway-clean-room]]  
**Entrypoint:** `verify/scripts/run-g034-shared-index-contention.sh` / compose `g034-shared-index-contention`

**Honesty:** Lease ≠ skip `index.lock`. ≠ M24 / G22 / G24 substitute. Host greens advisory. Product Verified = Docker PASS only. **G33 reserved elsewhere.**

| ID | Scenario | Expected | Host | Clean-room |
|----|----------|----------|------|------------|
| G34-C01 | 2 writers → one shared index under `index:shared` | Both paths land; lock released | `g034-shared-index-contention_c01_two_writer_shared_index` | required |
| G34-C02 | Second lease holder | `AlreadyHeld` | `g034-shared-index-contention_c02_second_holder_already_held` | required |
| G34-C03 | Live `index.lock` planted | Write fail-closed; no skip | `g034-shared-index-contention_c03_live_index_lock_refuses_write` | required |

Primary: [api-lockfile](https://git-scm.com/docs/api-lockfile), [gitformat-index](https://git-scm.com/docs/gitformat-index).
