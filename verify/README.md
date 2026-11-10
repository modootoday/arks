# arks clean-room verification harness (M1–M11 + G16 + G17)

**Authoritative:** Linux Docker containers from pinned images in `images.lock.json`, entered via `verify/scripts/run-g*.sh` (including `run-g6.sh`…`run-g15.sh`, `run-g16.sh`, `run-g17.sh`).

**Advisory:** `cargo test` on the host (including Windows) speeds iteration; it is not product-verified unless the same gate is green in clean-room. SoT: `.spec/sot/product-verification-clean-room.sot.md`.

**CLI-parity production** = **M9** / **G14** when evidenced. **G16** journey-compare is additive observability (func + timing); not a production unlock. **G17** is M12 concurrency issue-repro compare (git pathology vs arks resolved); Done only with clean-room per-ID PASS. SoT: `.spec/sot/clean-room-journey-compare.sot.md`, `.spec/sot/interop-test-matrix.sot.md`.

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

## Gates

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
| **G15** | Git server (M11) | `g15` |
| **G16** | Journey-compare catalog (func + timing metrics); additive | `g16` |
| **G17** | Concurrency issue-repro compare (M12 claimed set / Cm-1) | `g17` |

Each authoritative run should use a **fresh** container (`compose run --rm`, not a reused shell).

```bash
# from repo root
docker compose -f verify/docker-compose.yml run --rm g0
docker compose -f verify/docker-compose.yml run --rm g4
docker compose -f verify/docker-compose.yml run --rm g5
docker compose -f verify/docker-compose.yml run --rm g16
docker compose -f verify/docker-compose.yml run --rm g17
```

### Resource note

Set **`CARGO_BUILD_JOBS=2`** when sharing agents or CI runners (default in compose). Override only deliberately.

**Docker Desktop RAM (G16/G19):** journey-compare builds many temp repos per iteration. Give the Linux engine **≥8 GiB** RAM (Settings → Resources); **16 GiB** preferred when running G19. Do **not** overlap multiple `compose run … g16`/`g19` containers — concurrent catalogs OOMed hosts (exit **137**). Journey seeds in `verify/gates/g16/tests/journey_compare.rs` are capped for a **single** catalog under that floor; raise seeds only after raising Docker RAM.

### Target cache

Compose sets `CARGO_TARGET_DIR=/workspace/target-cleanroom` so clean-room artifacts stay separate from host `target/`.

**Agent / host hygiene:** `target-cleanroom/release/arks` is a **Linux ELF**. On Windows hosts:

- Prefer `target/release/arks.exe` for advisory runs (`ARKS_BIN=...`).
- **Do not** `Select-String`, `Get-Content`, or `findstr` binaries — dumps abort shells (`exit -1` / `4294967295`).
- Locate with metadata only: `pwsh -File verify/scripts/locate-arks.ps1` or `Get-Item` on known paths.

## Layout

```
verify/
  Dockerfile
  docker-compose.yml
  images.lock.json
  matrix-m1.md … matrix-m4.md
  scripts/run-g0.sh … run-g5.sh
  g1/ g2/ g4/ g5/ g10/   # gate oracles (not workspace members)
```

`verify/g*` harness crates are intentionally **outside** the root workspace so host default builds stay small; oracles run via their manifest paths.
