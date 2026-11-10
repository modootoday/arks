# L3 HighConcurrency stress matrix (M23 / G11 expansion)

**Authoritative runs:** `verify/docker-compose.yml` service `g11-l3` (`verify/scripts/run-g11-l3.sh`). Legacy alias: `g11l3` / `run-g11l3.sh` (historical evidence blobs keep that spelling). Host cargo tests are advisory.

**Claimed shape** ([[adr:root/20261109194200]]): lease + parallel prep + short ordered Git-lock batch apply + worktree/ref-shard isolation. **Shared-index N-way OUT.**

**Honesty:** lease ≠ skip Git locks. CELIR excluded. `WriterMode::HighConcurrency` valve unlocks only after dual-path receipts (Hc-3).

## Topology

| Tier | N writers | Isolation | Notes |
|------|----------:|-----------|-------|
| Smoke | 8 | Distinct `refs/heads/agent-NNN` shards | Host advisory OK |
| Stress-tens | 32 | Distinct refns shards | Clean-room required for Claimed |
| Stress-hundreds | 128 | Distinct refns shards | Clean-room required for Claimed |
| Negative | 8 | Same shared ref tip (no shard) | Pathology expected under serial locks |

## Pathology vs resolved

| ID | Induce | Stock Git / GitCompat pathology | arks HighConcurrency resolved (claimed shape) |
|----|--------|----------------------------------|-----------------------------------------------|
| L3-P01 | N agents CAS same `refs/heads/main` under exclusive locks | LockFailed / serialize / lost race | Not claimed — agents MUST use distinct shards |
| L3-R01 | N agents distinct `refs/heads/agent-*` + prep+batch | N/A (GitCompat serial OK but slow) | Parallel prep + ordered batch; all tips verify; locks still taken |
| L3-R02 | Lease held + competing `*.lock` on shard ref | LockFailed | Pipeline fails Pipeline(LockFailed); lease released; no skip |
| L3-R03 | WriterMode GitCompat default | — | Default path unchanged L0+L1/L2; HC opt-in after unlock |
| L3-R04 | Dual-path export honesty | — | Claimed apply writes stock-readable loose refs/objects (export = identity) |
| L3-X01 | Shared-index N-way | — | **OUT** — must not green |

## Cases (gate checklist)

| Case | Gate | How |
|------|------|-----|
| L3-C01 | g11-l3 | `prep_hash_parallel` + `apply_batch` N=32 tips_verified |
| L3-C02 | g11-l3 | N=128 shard apply tips_verified |
| L3-C03 | g11-l3 | Competing `*.lock` → Pipeline error; leases empty |
| L3-C04 | g11-l3 | GitCompat default; HighConcurrency opt-in accepted (post-unlock) |
| L3-C05 | g11-l3 | zero gix / zero celir metadata |
| L3-C06 | g11-l3 | G11 L1/L2 regress (`cargo test -p arks-lease`) |

## Done for Claimed

Docker `g11-l3` PASS + unlock evidence stamps residual L3 as Claimed; refuse removed for claimed shape only. Historical Claimed receipts may still cite compose id `g11l3`.
