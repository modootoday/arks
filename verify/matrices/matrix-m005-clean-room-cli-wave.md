# M5 clean-room usage-condition matrix (G6 / G3 + G1–G4 regression)

Roadmap **v0.3.1** M5 (Local CLI foundation / Phase L). Authoritative venue: Docker clean-room under `verify/` per [[sot:root/product-verification-clean-room]]. Image pin: `verify/images.lock.json` (`arks-verify-m1:0.1.0`). Depends on M4 [[evidence:root/20261108103610]]. Unit SoT: [[sot:root/arks]]. Wave L map: [[sot:root/cli-surface]] + [[plan:root/20261108090650]].

**Authoritative runs:** `verify/docker-compose.yml` services `g000-workspace-lib-tests`, `g006-cli-wave-l` (Wave L argv + embeds G3 + G1–G4 regression), `g003-index-commit-roundtrip` (standalone full index-commit). Stock Git in-image is the peer oracle. Pack helpers remain library-only (no Wave L pack bins).

| ID | Condition | Gate | Automated | Entry |
|----|-----------|------|-----------|-------|
| M5-C01 | `arks init` → stock `git status` / `git rev-parse --git-dir` opens repo | G6 | yes | `verify/gates/g006-cli-wave-l` |
| M5-C02 | `arks hash-object -w` ↔ `git hash-object -w` OID parity | G6 / G1 | yes | `verify/gates/g006-cli-wave-l` (+ G1 regression) |
| M5-C03 | `arks cat-file -t/-s/-p` ↔ stock `git cat-file` | G6 | yes | `verify/gates/g006-cli-wave-l` |
| M5-C04 | `arks update-ref` + `arks rev-parse` ↔ `git show-ref` / `git rev-parse` | G6 / G2 | yes | `verify/gates/g006-cli-wave-l` |
| M5-C05 | `arks add` + `arks status` index staging vs `git ls-files` / `git status` | G6 / G3 | yes | `verify/gates/g006-cli-wave-l` / `verify/gates/g003-index-commit-roundtrip` |
| M5-C06 | `arks commit -m` → `git log` / `git cat-file -p HEAD` readable | G3 | yes | `verify/gates/g003-index-commit-roundtrip` |
| M5-C07 | Reverse: `git add`/`git commit` → `arks log` / `arks show` / `arks status` | G3 | yes | `verify/gates/g003-index-commit-roundtrip` |
| M5-C08 | `arks log` / `arks show` first-parent walk | G6 | yes | `verify/gates/g006-cli-wave-l` |
| M5-C09 | Help lists only Wave L commands; unsupported flags refuse non-zero | G6 | yes | `verify/gates/g006-cli-wave-l` |
| M5-C10 | Workspace / `arks` unit tests | G0 | yes | `run-g000-workspace-lib-tests.sh` |
| M5-C11 | G1 objects regression | G1 | yes | via `run-g006-cli-wave-l.sh` |
| M5-C12 | G2 refs regression | G2 | yes | via `run-g006-cli-wave-l.sh` |
| M5-C13 | G4 pack regression | G4 | yes | via `run-g006-cli-wave-l.sh` |
| M5-C14 | G10 facade subset still green | G10 | yes | via `run-g006-cli-wave-l.sh` |
| M5-C15 | Remotes / Wave H | — | no | Deferred to M6 |
| M5-C16 | CELIR / lease L1+ | — | no | Out of M5 |
| M5-C17 | Pa/Pb/Pc porcelain inventory | — | no | Deferred to M7–M9 |

Legend: **G6** = Wave L CLI argv vs stock Git; **G3** = full index+commit round-trip; regressions embed prior gates.

Host-only green does **not** carry M5 exit.
