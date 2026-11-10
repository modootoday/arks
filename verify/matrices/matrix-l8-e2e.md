# L8 product-completion e2e suite (clean-room Docker)

Owning ladder: `.spec/research/20261110095142-product-completion-goal-ladder.research.md` (**L8**).
Venue: Docker clean-room only (`verify/docker-compose.yml`). Host `cargo test` is advisory — never invent Verified/PASS from host green.

## Suite (sequential; one `compose run --rm` at a time)

| Order | Compose | Intent | Required for L8 | Tip status (agent receipts under `out/`; not Claimed invent) |
|-------|---------|--------|-----------------|--------------------------------------------------------------|
| 1 | `g14` | M9 CLI-parity production + G13→G1 regression | **yes** | Docker PASS on tip |
| 2 | `g16` | 19 diverse git↔arks journeys (func + timing metrics) | **yes** | Docker PASS (19 journeys) on tip |
| 3 | `g20` | M15 full CLI inventory symmetry | expand until green | Docker PASS after celir default-tree scan fix |
| 4 | `g21` | M16 complete flag / alias symmetry | expand until green | Docker PASS (sy5 bisect fixture outside WT) |

Optional later layers (still sequential; not a substitute for 1–2): `g7`, `g15`, `g35`, `g37`. Do **not** parallelize with `g19`. Do **not** restart sole G19 remasure unless honesty requires it.

## Anti-stampede

- Default compose `CARGO_BUILD_JOBS=1` and named volume `arks-cargo-target`.
- Fresh container per gate (`run --rm`).
- Halt on OOM / exit 137; reduce parallelism; do not invent green.

## Entrypoint

```bash
# from repo root
bash verify/scripts/run-l8-e2e.sh
```

L8 Done only when required gates report PASS on tip in Docker receipts (and expanded gates if declared in this matrix).
