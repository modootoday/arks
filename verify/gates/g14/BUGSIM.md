# G14 bug simulation notes

1. `verify/scripts/run-g14.sh` MUST be LF-only. CRLF yields `set: pipefail` / `$'\r'` failures (same class as G7/G12/G13).
2. Alias shim tests create a temporary `PATH` directory with an executable `git` script that `exec`s the arks binary — do not rely on host shell `alias` inside non-interactive cargo tests.
3. Host advisory `arks_bin()` honors `CARGO_TARGET_DIR` (clean-room) and prefers workspace `target/debug/arks(.exe)` on host (avoids Win32 error 193 from Linux ELF under `target-cleanroom`).
