# M2 clean-room usage-condition matrix (G0 / G1 / G10)

Roadmap **v0.3.1** M2 **restart / re-verify**: rows below define **claimed automated** combinatorics (M2-C01…C14 unchanged). Carrying clean-room receipt: [[evidence:root/20261108190033]] (`.spec/sot/product-verification-clean-room.sot.md`). Host-only green and historical [[evidence:root/20261108093059]] alone do **not** carry exit. Depends on M1 restart [[evidence:root/20261108185319]].

**Authoritative runs:** `verify/docker-compose.yml` services `g0`, `g1`, `g10`. Stock Git in-image only. Image pin: `verify/images.lock.json` (`arks-verify-m1:0.1.0`).

| ID | Condition | Gate | Automated | Entry |
|----|-----------|------|-----------|-------|
| M2-C01 | Empty blob OID (SHA-1) via facade `put` | G10 | yes | `verify/gates/g10` `empty_blob_oid_matches_git_hash_object` |
| M2-C02 | `hello` blob OID (SHA-1) via facade `put` | G10 | yes | `verify/gates/g10` `hello_blob_oid_matches_git_hash_object` |
| M2-C03 | Facade `put` → `get` roundtrip (blob payload) | G10 | yes | `verify/gates/g10` `facade_put_get_roundtrip` |
| M2-C04 | Facade `exists` true after `put` | G10 | yes | `verify/gates/g10` `facade_exists_true_after_put` |
| M2-C05 | Facade `exists` false for absent OID | G10 | yes | `verify/gates/g10` `facade_exists_false_absent` |
| M2-C06 | Facade write → `git cat-file -p` | G10 | yes | `verify/gates/g10` `facade_write_git_cat_file_roundtrip` |
| M2-C07 | `git hash-object -w` → facade `get` | G10 | yes | `verify/gates/g10` `git_hash_object_write_facade_get_roundtrip` |
| M2-C08 | M1 G1 oracle subset (workspace) | G1 | yes | `arks-object` `--test g1_oracle` (via `run-g1.sh` / `run-g10.sh`) |
| M2-C09 | M1 stock-Git oracle (verify/gates/g1) | G1 | yes | `verify/gates/g1` `git_oracle.rs` (via `run-g1.sh` / `run-g10.sh`) |
| M2-C10 | Workspace library tests (G0) | G0 | yes | `cargo test --workspace` via `run-g0.sh` |
| M2-C11 | `arks-store` facade unit tests | G0 | partial | Included in G0 when `crates/arks-store` is a workspace member |
| M2-C12 | G1–G4-through-facade (refs/pack) | — | no | Post-M4; interop matrix phased G10 |
| M2-C13 | CELIR-backed store path | G10 | no | Blocked: [[sot:root/storage-celir-boundary]] not ready; no harness automation |
| M2-C14 | Lease / G11 | — | no | Out of M2 scope |

Legend: **G0** = `cargo test --workspace`; **G1** = M1 oracle suites; **G10** = `verify/gates/g10` facade-over-loose-objects vs stock `git`.

Regression: **G10 authoritative entry** (`run-g10.sh`) runs **G10** then **G1** (M2-C08–C09) so facade work cannot regress loose-object interop.
