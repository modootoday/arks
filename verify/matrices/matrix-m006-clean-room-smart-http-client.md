# M6 clean-room usage-condition matrix (G7 + G1–G6 regression)

Roadmap **v0.3.1+** M6 (Remote client / Phase H). Authoritative venue: Docker clean-room under `verify/` per [[sot:root/product-verification-clean-room]]. Image pin: `verify/images.lock.json` (`arks-verify-m1:0.1.0`). Depends on M5 [[evidence:root/20261108104837]]. Unit SoT: [[sot:root/arks-remote]] + Wave H in [[sot:root/arks]]. Plan: [[plan:root/20261108090651]]. Remotes: generic smart-HTTP client; GitHub-class = verify peer label only ([[sot:root/remote-host-compatibility-surface]]).

**Authoritative runs:** `verify/docker-compose.yml` service `g007-smart-http-client` (`verify/scripts/run-g007-smart-http-client.sh`). G7 uses **offline / loopback** smart-HTTP peers (recorded or local stock `git http-backend`) — no live forge network required. **G8** live smoke default **Refuse-closed** ([[adr:root/20261110151000]]); opt-in via `g008-live-forge-smoke` service + secrets. **G9** Claimed `--depth` subset lands under compose **G35** (`verify/matrices/matrix-g035-shallow-depth.md`); filter/promisor still N/A.

| ID | Condition | Gate | Automated | Entry |
|----|-----------|------|-----------|-------|
| M6-C01 | `arks remote add/list/remove` writes `.git/config` remotes stock Git can read | G7 | yes | `verify/gates/g007-smart-http-client` |
| M6-C02 | `arks clone` via smart HTTP from local/recorded peer → objects+refs+baseline checkout | G7 | yes | `verify/gates/g007-smart-http-client` |
| M6-C03 | `arks fetch` updates remote-tracking refs from peer | G7 | yes | `verify/gates/g007-smart-http-client` |
| M6-C04 | `arks push` updates peer refs; `report-status` ok for claimed subset | G7 | yes | `verify/gates/g007-smart-http-client` |
| M6-C04b | `arks submodule add` over loopback `http(s)://` smart HTTP (FW-SUBMOD-HTTPS) | G7 | yes | `verify/gates/g007-smart-http-client` `submodule_add_https_smart_http_loopback` |
| M6-C05 | Help lists Wave L+H claimed commands; `pull` / SSH refuse non-zero | G7 | yes | `verify/gates/g007-smart-http-client` |
| M6-C06 | Generic peer (not GitHub-only code path); GitHub-class may appear as fixture label only | G7 | yes | `verify/gates/g007-smart-http-client` |
| M6-C07 | Workspace / `arks-remote` + `arks` unit tests | G0 | yes | `run-g000-workspace-lib-tests.sh` via `run-g007-smart-http-client.sh` |
| M6-C08 | G6 Wave L regression | G6 | yes | via `run-g007-smart-http-client.sh` |
| M6-C09 | G3 index-commit regression | G3 | yes | via `run-g007-smart-http-client.sh` |
| M6-C10 | G5 protocol offline regression | G5 | yes | via `run-g007-smart-http-client.sh` |
| M6-C11 | G4 pack regression | G4 | yes | via `run-g007-smart-http-client.sh` |
| M6-C12 | G2 refs regression | G2 | yes | via `run-g007-smart-http-client.sh` |
| M6-C13 | G10 facade subset regression | G10 | yes | via `run-g007-smart-http-client.sh` |
| M6-C14 | G1 objects regression | G1 | yes | via `run-g007-smart-http-client.sh` |
| M6-C15 | gix-exit (zero gix/gitoxide edges) | — | yes | `run-g007-smart-http-client.sh` metadata scan |
| M6-C16 | G8 live GitHub-class smoke | G8 | yes (refuse default / live opt-in) | **Refuse-closed** default; `run-g008-live-forge-smoke.sh` |
| M6-C17 | G9 `--depth` shallow (filter N/A) | G9 / **G35** | yes | `verify/matrices/matrix-g035-shallow-depth.md` / compose `g035-shallow-depth` |
| M6-C18 | SSH remotes | — | no | Not claimed (refuse) |
| M6-C19 | Server role / forge APIs / CELIR / lease L1+ | — | no | Out of M6 |

Legend: **G7** = remote-recorded / loopback smart-HTTP client proof; regressions embed prior gates.

Host-only green does **not** carry M6 exit.
