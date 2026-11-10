# G20 bug simulation notes

1. `verify/scripts/run-g020-cli-symmetric-parity.sh` MUST be LF-only. CRLF yields `set: pipefail` / `$'\r'` failures.
2. Host advisory `ARKS_BIN` must point at a native PE (`target/release/arks.exe` on Windows). Do not execute Linux ELF from `target-cleanroom/` on the host (Win32 error 193).
3. Bind `Command::Output` before `String::from_utf8_lossy(&out.stdout)` — temporary `arks(...).stdout` borrows fail with **E0716** and abort the harness compile (exit 101).
4. **Never** `Select-String` / `Get-Content` / `findstr` Linux ELF or PE binaries under `target*/`. Use `Get-Item` metadata or `pwsh -File verify/scripts/locate-arks.ps1` (metadata + magic-byte kind only). Binary dumps abort agent shells with exit `-1` / `4294967295`.
