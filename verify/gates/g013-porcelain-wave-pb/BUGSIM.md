# G13 harness bug simulation (avoided)

Same class as G12 / M6 `run-g007-smart-http-client.sh`:

1. A `verify/scripts/run-g013-porcelain-wave-pb.sh` written with CRLF on Windows fails `set -euo pipefail` in the Linux clean-room (`set: pipefail`). The script is LF-only (`verify/.gitattributes`: `*.sh text eol=lf`).
2. Host advisory `arks_bin()` must not prefer `target-cleanroom/debug/arks` (Linux ELF) on Windows, which yields Win32 error 193. The helper honors `CARGO_TARGET_DIR` when set (clean-room) and otherwise prefers workspace `target/debug/arks(.exe)`.
