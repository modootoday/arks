# arks clean-room verification harness

**Authoritative:** Linux Docker containers from pinned images in `images.lock.json`, entered via `verify/scripts/run-g*.sh`.

**Advisory:** `cargo test` on the host (including Windows) speeds iteration; it is not product-verified unless the same gate is green in clean-room.

SoTs: `.spec/sot/product-verification-clean-room.sot.md`, `.spec/sot/verify-directory-conventions.sot.md` (tree / folder / filename / **numeric** notation / out / SKIP·STRICT — §2a–§2b), `.spec/sot/structural-conventions.sot.md` §7, research `.spec/research/20261110095355-verify-fixtures-rust-layout.research.md`, `.spec/research/20261111082440-verify-directory-conventions-pain.research.md`.

**CLI-parity production** = **M9** / **G14** when evidenced. **G16** journey-compare is additive observability (func + timing). **G19** enforces comparative allow-list bars on G16 metrics. **G17** is M12 concurrency issue-repro compare. SoT: `.spec/sot/clean-room-journey-compare.sot.md`, `.spec/sot/interop-test-matrix.sot.md`.

## Engine affinity (Windows)

Use **Docker Desktop → Linux engine** (`desktop-linux`). Do not substitute an undeclared WSL-only daemon. Humanymous cites: `96-docker-engine-affinity`, `60-e2e-docker-only`.

## Images

- Recipe: `verify/Dockerfile` (Rust **1.99.1** on Debian bookworm + apt `git`).
- Pins: `verify/images.lock.json` (base digest + built `arks-verify-m1` digest).
- Expected stock Git in image: Debian bookworm apt Git (see lock `stock_git`).

Build once:

```bash
docker compose -f verify/docker-compose.yml build
```

Refresh lock digests after rebuild: `docker inspect arks-verify-m1:0.1.0 --format '{{index .RepoDigests 0}}'`.

## Layout (matrix ownership)

Normative detail: `.spec/sot/verify-directory-conventions.sot.md`.

```text
verify/
  Dockerfile
  docker-compose.yml
  images.lock.json
  README.md
  scripts/                 # deterministic entrypoints (compose commands)
  matrices/                # matrix-m*.md / matrix-g*.md / matrix-journey.md / *-matrix.csv
  gates/gN/                # per-gate oracles, locks, out/ receipts (not workspace members)
  fixtures/                # shared oracle intent/bytes (no secrets); m1 OID docs
  out/                     # gitignored cross-gate inventory / prerelease dumps
```

| Tree | Owns | Does not own |
|------|------|--------------|
| `verify/scripts/` | Compose argv entrypoints, evaluators | Crate unit tests |
| `verify/matrices/` | Gate/milestone claim matrices + living symmetry CSVs | Specfold SoTs |
| `verify/gates/gN/` | Multi-crate / stock-Git / journey clean-room oracles + receipts | Default host `cargo test -p` |
| `verify/fixtures/` | Clean-room shared fixture intent | Crate-private `testdata/` |
| `verify/out/` | Cross-gate inventory / prerelease dumps (gitignored) | Per-gate Claimed receipts |
| `crates/arks-*/tests/` | Single-crate integration (host-advisory; scripts may invoke) | Docker image pins |
| `crates/arks-*/testdata/` | Crate-owned binary/json fixtures | Product matrices |

### Naming families

Folder ids under `verify/gates/<id>/` are kebab-case and should match compose service names (`gN`, `gN-<suffix>`, `fsNNN-*-advisory`; reserved `dod-*` only when a gate oracle exists); file basenames (`run-*.sh`, `matrix-*`, receipts, compose/lock) follow the same SoT §2a; digit width / zero-pad (`g11` unpadded vs `fs007` three-digit) follow §2b — see `.spec/sot/verify-directory-conventions.sot.md` §1a–§1d / §2a–§2b. Migration: live id `g11-l3` (legacy `g11l3`); evaluator `g19-eval.py`. Do **not** zero-pad G-series or un-pad `fsNNN`.

| Family | Compose | Scripts |
|--------|---------|---------|
| Product / milestone gates | `gN`, `gN-<suffix>` | `run-gN.sh` |
| Symmetry DoD / release / hosting | `dod-<concern>` | `run-*-matrix-e2e.sh`, `run-symmetry-*.sh`, `run-self-host-e2e.sh` |
| Claimed residual (B-slots) | `dod-claimed-<concern>`, `dod-claimed-residual` | `run-claimed-*-e2e.sh` |
| Host advisory | `fsNNN-*-advisory` | `run-fsNNN-*-advisory.sh` |

Default scaffold / SKIP exit 0; `ARKS_DOD_STRICT=1` fails closed — smoke ≠ Claimed. Stock Git pin **2.39.5** in-image is normative; host Git is advisory only.

`verify/gates/g*` harness crates stay **outside** the root workspace so host default builds stay small; oracles run via `--manifest-path verify/gates/gN/Cargo.toml` or `cargo test -p …` from scripts.

**Repo-root path depth:** gate `CARGO_MANIFEST_DIR` is `verify/gates/gN/`, so host fallbacks to repo `target/` / `target-cleanroom/` use `../../..` (three parents). Prefer `CARGO_TARGET_DIR` / `ARKS_BIN` in clean-room.

### G19 paths (sole Docker run)

| Artifact | Path |
|----------|------|
| Entrypoint | `verify/scripts/run-g19.sh` → `run-g16.sh` + `g19-eval.py` |
| Journey suite | `verify/gates/g16/` |
| Metrics | `verify/gates/g16/out/metrics.json` |
| Receipt / lock | `verify/gates/g19/out/receipt.json`, `verify/gates/g19/.run.lock.dir` |
| Matrices | `verify/matrices/matrix-g19.md`, `verify/matrices/matrix-journey.md` |

## Gates (selected)

| Gate | Intent | Compose |
|------|--------|---------|
| **G0** | Workspace library tests | `g0` |
| **G1** | Stock-Git oracle vs arks loose objects | `g1` |
| **G2** | Refs round-trip + index r/w vs stock Git + G10/G1 regression | `g2` |
| **G4** | Pack/idx local round-trip vs stock Git; embeds G5 + G2/G10/G1 | `g4` |
| **G5** | Protocol offline fixtures (pkt-line / capabilities) | `g5` |
| **G10** | Store facade (`FilesystemGitStore`) vs stock Git + G1 regression | `g10` |
| **G6** | Wave L local CLI argv + regressions | `g6` |
| **G7** | Wave H smart-HTTP remote client + G6…G1 regression; gix-exit | `g7` |
| **G12** | Wave Pa porcelain + G7/G6…G1 regression; gix-exit | `g12` |
| **G13** | Wave Pb porcelain + G12/G7/G6…G1 regression; gix-exit | `g13` |
| **G14** | Wave Pc inventory + flag/alias + same-`.git` + G13…G1 regression; gix-exit — **CLI-parity production** | `g14` |
| **G11** | Lease product gates (M10) | `g11` |
| **G11-L3** | M23 HighConcurrency claimed-shape stress (G11 expansion) | `g11-l3` (legacy `g11l3`) |
| **G15** | Git server (M11) | `g15` |
| **G16** | Journey-compare catalog (func + timing metrics); additive | `g16` |
| **G17** | Concurrency issue-repro compare (M12 claimed set / Cm-1) | `g17` |
| **G19** | Comparative allow-list bars on G16 metrics | `g19` |

Each authoritative run should use a **fresh** container (`compose run --rm`, not a reused shell).

```bash
# from repo root
docker compose -f verify/docker-compose.yml run --rm g0
docker compose -f verify/docker-compose.yml run --rm g4
docker compose -f verify/docker-compose.yml run --rm g5
docker compose -f verify/docker-compose.yml run --rm g16
docker compose -f verify/docker-compose.yml run --rm g19
```

### Product-completion e2e (L8)

Sequential suite (do not parallelize with `g16`/`g19`):

```bash
# from repo root — mkdir out/ first on Windows before host Tee/redirects
mkdir -p verify/gates/g14/out verify/gates/g16/out
bash verify/scripts/run-l8-e2e.sh
# or: docker compose -f verify/docker-compose.yml run --rm g14
# then: docker compose -f verify/docker-compose.yml run --rm g16
```

Matrix: `verify/matrices/matrix-l8-e2e.md`. Comparison Pages draft: `docs/git-vs-arks.md`.

### Resource note

Compose defaults **`CARGO_BUILD_JOBS=1`**. Raise only deliberately. On Windows hosts, create `verify/gates/<gate>/out/` before `Tee-Object` / file redirects — missing dirs abort the wrapper with exit `4294967295` before Docker evidence lands.

**Docker Desktop RAM (G16/G19):** journey-compare builds many temp repos per iteration. Give the Linux engine **≥8 GiB** RAM (Settings → Resources); **16 GiB** preferred when running G19. Do **not** overlap multiple `compose run … g16`/`g19` containers — concurrent catalogs OOMed hosts (exit **137**). Journey seeds in `verify/gates/g16/tests/journey_compare.rs` are capped for a **single** catalog under that floor; raise seeds only after raising Docker RAM.

### Target cache

Compose uses a named volume (`arks-cargo-target` → `/cargo-target`) and sets `CARGO_TARGET_DIR=/cargo-target` so clean-room artifacts stay separate from host `target/`.

**Agent / host hygiene:** clean-room `arks` under that volume is a **Linux ELF**. On Windows hosts:

- Prefer `target/release/arks.exe` for advisory runs (`ARKS_BIN=...`).
- **Do not** `Select-String`, `Get-Content`, or `findstr` binaries — dumps abort shells (`exit -1` / `4294967295`).
- Locate with metadata only: `pwsh -File verify/scripts/locate-arks.ps1` or `Get-Item` on known paths.
