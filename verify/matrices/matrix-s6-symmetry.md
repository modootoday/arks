# S6 symmetry e2e suite (clean-room Docker)

Owning ladder: `.spec/research/20261110105436-git-symmetry-expanded-dod-goal-ladder-s0-s-final.research.md` (**S6**).
Venue: Docker clean-room only (`verify/docker-compose.yml`). Host cargo/gpg is **not** Verified. Never invent PASS.

## Suite (sequential; one `compose run --rm --no-deps` at a time)

| Order | Compose | Intent | Required for S6 | Notes |
|-------|---------|--------|-----------------|-------|
| — | L8 (`g14`→`g16`→`g20`→`g21`) | Prior git↔arks e2e | **cite Done** | `verify/matrices/matrix-l8-e2e.md`; re-run only if `ARKS_S6_INCLUDE_L8=1` |
| 1 | `g38` | R4 OpenPGP / X.509 / `tag -s` | **yes** | matrix-g38.md |
| 2 | `g39` | R3 `blob:none` twin | **yes** | matrix-g39.md |
| 3 | `g40` | S3 dumb HTTP deletes twin | **yes** | matrix-g40.md |
| opt | `g35` | `--depth` related peer | optional | set `ARKS_S6_INCLUDE_G35=1` |

## Anti-stampede

- Compose default `CARGO_BUILD_JOBS=1` + volume `arks-cargo-target`.
- Fresh container per gate; `--no-deps` after image exists.
- `mkdir -p verify/gates/{g38,g39,g40}/out` before host Tee.
- Halt on OOM / exit 137; do not parallelize with G19 remasure.

## Entrypoint

```bash
# from repo root
bash verify/scripts/run-s6-symmetry.sh
```
