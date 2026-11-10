# M3 clean-room usage-condition matrix (G0 / G1 / G2 / G10)

Roadmap **v0.3.1** M3 (Refs + index + repo / Phase R). Authoritative venue: Docker clean-room under `verify/` per [[sot:root/product-verification-clean-room]]. Image pin: `verify/images.lock.json` (`arks-verify-m1:0.1.0`). Depends on M2 restart [[evidence:root/20261108190033]].

**Authoritative runs:** `verify/docker-compose.yml` services `g000-workspace-lib-tests`, `g002-refs-index-roundtrip` (G2 includes G10/G1 regression). Stock Git in-image only.

| ID | Condition | Gate | Automated | Entry |
|----|-----------|------|-----------|-------|
| M3-C01 | arks `update` loose ref → `git show-ref` | G2 | yes | `verify/gates/g002-refs-index-roundtrip` `arks_update_ref_readable_by_git_show_ref` |
| M3-C02 | `git update-ref` → arks `resolve` | G2 | yes | `verify/gates/g002-refs-index-roundtrip` `git_update_ref_readable_by_arks` |
| M3-C03 | arks HEAD symref → `git rev-parse HEAD` | G2 | yes | `verify/gates/g002-refs-index-roundtrip` `arks_head_symref_matches_git_rev_parse` |
| M3-C04 | arks delete loose ref → absent in `git show-ref` | G2 | yes | `verify/gates/g002-refs-index-roundtrip` `arks_delete_ref_absent_from_git_show_ref` |
| M3-C05 | L0 ref.lock contention fails update | G2 | yes | `verify/gates/g002-refs-index-roundtrip` `l0_ref_lock_contention_fails` |
| M3-C06 | `arks-repo` discover + HEAD resolve | G2 | yes | `verify/gates/g002-refs-index-roundtrip` `repo_compose_discovers_and_resolves_head` |
| M3-C07 | arks v2 index write → `git ls-files --stage` | G2 | yes | `verify/gates/g002-refs-index-roundtrip` `arks_index_write_readable_by_git_ls_files` |
| M3-C08 | `git add` index → arks `Index::read_path` | G2 | yes | `verify/gates/g002-refs-index-roundtrip` `git_add_index_readable_by_arks` |
| M3-C09 | L0 index.lock contention fails write | G2 | yes | `verify/gates/g002-refs-index-roundtrip` `l0_index_lock_contention_fails` |
| M3-C10 | Workspace library tests (incl. refs/index/repo) | G0 | yes | `cargo test --workspace` via `run-g000-workspace-lib-tests.sh` |
| M3-C11 | M1 G1 oracle subset (workspace) | G1 | yes | `arks-object` `--test g001-loose-object-interop_oracle` (via `run-g002-refs-index-roundtrip.sh`) |
| M3-C12 | M1 stock-Git oracle (verify/gates/g001-loose-object-interop) | G1 | yes | `verify/gates/g001-loose-object-interop` (via `run-g002-refs-index-roundtrip.sh`) |
| M3-C13 | M2 G10 facade oracle | G10 | yes | `verify/gates/g010-store-facade` (via `run-g002-refs-index-roundtrip.sh`) |
| M3-C14 | packed-refs write / rewrite | — | no | Out of M3 (read-only packed-refs) |
| M3-C15 | Full G3 index-commit round-trip | G3 | no | Deferred to M5 |
| M3-C16 | CELIR path | — | no | Blocked: storage-celir-boundary |
| M3-C17 | Lease L1+ / G11 | — | no | Out of M3 (L0 only) |

Legend: **G2** = `verify/gates/g002-refs-index-roundtrip` refs + index r/w vs stock `git`; **G0** = workspace tests; regression embeds G10+G1.

Host-only green does **not** carry M3 exit.
