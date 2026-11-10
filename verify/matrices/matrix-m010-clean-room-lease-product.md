# M10 clean-room matrix (G11 lease-product)

| ID | Scenario | Gate | Expect |
|----|----------|------|--------|
| M10-C01 | L1 grant / release / inspect / list | G11 | `cargo test -p arks-lease` |
| M10-C02 | L1 expire fencing | G11 | expire_due drops stale |
| M10-C03 | L2 durable round-trip under `.git/arks/leases/` | G11 | reload sees grants |
| M10-C04 | Dual-path GitCompat default | G11 | WriterSession default |
| M10-C05 | HighConcurrency dual-path valve (default GitCompat; HC opt-in) | G11 | M10 history refuse; M23 Claimed [[evidence:root/20261109194858]] |
| M10-C06 | Lease held + competing `*.lock` still LockFailed | G11 | lease ≠ skip Git locks |
| M10-C07 | zero gix / zero celir | G11 | cargo metadata scan in run-g011-lease-product.sh |
| M10-C08 | G14 + G7 regression | G11 | embedded prior suites |
| M10-C09 | No forge / no CELIR in help | G11 | verify/gates/g011-lease-product help smoke |

Residual disposition: L3 claimed shape **M23** ([[adr:root/20261109194200]] / `verify/matrices/matrix-l003-high-concurrency.md`); M10 history refuse-closed ([[adr:root/20261108232800]]); allocator identity **Closed** ring-internal ([[adr:root/20261109122126]]).
