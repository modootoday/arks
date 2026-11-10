# G16 bug simulation notes

1. `verify/scripts/run-g16.sh` MUST be LF-only. CRLF yields `set: pipefail` / `$'\r'` failures (same class as G14/G13).
2. Authoritative timings use **release** `arks` (`cargo build -p arks --release`). `arks_bin()` must prefer `release/` under `CARGO_TARGET_DIR` before `debug/`.
3. Alias shim uses a temporary PATH `git` script that `exec`s arks — not shell `alias` inside cargo tests.
4. Host advisory runs may pick host `target/release/arks(.exe)`; do not execute Linux ELF from `target-cleanroom` on Windows.
5. Compile/link time is excluded from metrics; build before timed journeys.
6. **Never** `Select-String` / `Get-Content` binary ELFs under `target-cleanroom/` (or PE under `target/`). Locate with `Get-Item` / `verify/scripts/locate-arks.ps1` only — content dumps abort shells (`exit -1` / `4294967295`).
7. `J-CLONE-LOCAL-01` is functionally required on G16; comparative ≥20% for that ID remains **Open** for G19 Pf-3+ (run-to-run ratio noise) — do not treat flaky G19 allow-list expansion as a G16 functional abort.
