# S6 symmetry e2e suite (clean-room Docker)

Owning ladder: `.spec/research/20261110105436-git-symmetry-expanded-dod-goal-ladder-s0-s-final.research.md` (**S6**).
Venue: Docker clean-room only (`verify/docker-compose.yml`). Host cargo/gpg is **not** Verified. Never invent PASS.

## Suite (sequential; one `compose run --rm --no-deps` at a time)

| Order | Compose | Intent | Required for S6 | Notes |
|-------|---------|--------|-----------------|-------|
| — | L8 (`g014-cli-parity-production`→`g016-journey-compare`→`g020-cli-symmetric-parity`→`g021-cli-flag-symmetry`) | Prior git↔arks e2e | **cite Done** | `verify/matrices/matrix-l008-product-completion-e2e.md`; re-run only if `ARKS_S6_INCLUDE_L8=1` |
| 1 | `g038-gpgsig-openpgp` | R4 OpenPGP / X.509 / `tag -s` | **yes** | matrix-g038-gpgsig-openpgp.md |
| 2 | `g039-partial-clone-filter` | R3 `blob:none` twin | **yes** | matrix-g039-partial-clone-filter.md |
| 3 | `g040-dumb-http-push-deletes` | S3 dumb HTTP deletes twin | **yes** | matrix-g040-dumb-http-push-deletes.md |
| opt | `g035-shallow-depth` | `--depth` related peer | optional | set `ARKS_S6_INCLUDE_G35=1` |

## Anti-stampede

- Compose default `CARGO_BUILD_JOBS=1` + volume `arks-cargo-target`.
- Fresh container per gate; `--no-deps` after image exists.
- `mkdir -p verify/gates/{g038-gpgsig-openpgp,g039-partial-clone-filter,g040-dumb-http-push-deletes}/out` before host Tee.
- Halt on OOM / exit 137; do not parallelize with G19 remasure.

## Entrypoint

```bash
# from repo root
bash verify/scripts/run-s006-symmetry-e2e.sh
```
