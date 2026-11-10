# G12 harness bug simulation (fixed)

1. `verify/scripts/run-g012-porcelain-wave-pa.sh` was written with CRLF on Windows → would fail `set -euo pipefail` in Linux clean-room (`set: pipefail`). Rewritten to LF (same class of bug as M6 `run-g007-smart-http-client.sh`).
2. Host advisory `arks_bin()` initially preferred `target-cleanroom/debug/arks` (Linux ELF from prior compose runs) on Windows → Win32 error 193. Fixed to honor `CARGO_TARGET_DIR` when set (clean-room) and otherwise prefer workspace `target/debug/arks(.exe)`.
