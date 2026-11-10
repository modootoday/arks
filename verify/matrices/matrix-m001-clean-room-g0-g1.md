# M1 clean-room usage-condition matrix (G0 / G1)

Roadmap **v0.3.1** M1 **restart / re-verify**: rows below define **claimed automated** combinatorics; product exit needs a **new** clean-room receipt (`.spec/sot/product-verification-clean-room.sot.md`), not host-only green.

**Authoritative runs:** `verify/docker-compose.yml` services `g000-workspace-lib-tests`, `g001-loose-object-interop`; image pin `verify/images.lock.json`. Stock Git in-image only.

| ID | Condition | Gate | Automated | Entry |
|----|-----------|------|-----------|-------|
| M1-C01 | Empty blob OID (SHA-1) | G1 | yes | `verify/scripts/run-g001-loose-object-interop.sh` → `verify/gates/g001-loose-object-interop` `empty_blob_oid_matches_git_hash_object` |
| M1-C02 | `hello` blob OID (SHA-1) | G1 | yes | `verify/scripts/run-g001-loose-object-interop.sh` → `verify/gates/g001-loose-object-interop` `hello_blob_oid_matches_git_hash_object` |
| M1-C03 | zlib encode/decode roundtrip (in-memory) | G0 | yes | `verify/scripts/run-g000-workspace-lib-tests.sh` → `arks-object` unit `blob_roundtrip_memory` / `zlib_roundtrip_preserves_bytes` |
| M1-C04 | loose write/read on disk (arks only) | G0 | yes | `verify/scripts/run-g000-workspace-lib-tests.sh` → `arks-object` unit `loose_write_read_roundtrip` |
| M1-C05 | arks loose write → `git cat-file -p` | G1 | yes | `verify/scripts/run-g001-loose-object-interop.sh` → `verify/gates/g001-loose-object-interop` `arks_write_git_cat_file_roundtrip` |
| M1-C06 | `git hash-object -w` → arks `read_loose` | G1 | yes | `verify/scripts/run-g001-loose-object-interop.sh` → `verify/gates/g001-loose-object-interop` `git_hash_object_write_arks_read_roundtrip` |
| M1-C07 | SHA-256 object format digests (API only) | G0 | yes | `verify/scripts/run-g000-workspace-lib-tests.sh` → `arks-hash` unit `empty_blob_sha256_length_and_oid` (+ related SHA-256 units; **no** stock-Git G1 oracle) |
| M1-C08 | tree / commit / tag **stock-Git payload** loose objects | — | no | Deferred; opaque kind framing is M1-C11 only |
| M1-C09 | pack indexes / thin packs | — | no | out of M1 scope |
| M1-C10 | remote / GitHub-class smoke | — | no | network policy open in SoT |
| M1-C11 | Claimed kinds (`blob`/`tree`/`commit`/`tag`) opaque payload encode/decode + OID bind | G0 | yes | `verify/scripts/run-g000-workspace-lib-tests.sh` → `arks-object` unit `claimed_kinds_opaque_payload_roundtrip` |
| M1-C12 | Binary NUL blob OID + arks→git roundtrip | G1 | yes | `verify/scripts/run-g001-loose-object-interop.sh` → `verify/gates/g001-loose-object-interop` `binary_nul_blob_oid_matches_git` (+ `arks-object` `g001-loose-object-interop_oracle` `binary_nul_blob_oid_and_roundtrip`) |
| M1-C13 | Empty blob bidirectional loose (arks↔git) | G1 | yes | `verify/scripts/run-g001-loose-object-interop.sh` → `verify/gates/g001-loose-object-interop` `empty_blob_arks_write_git_cat_file` (+ `g001-loose-object-interop_oracle` `empty_blob_bidirectional_loose`) |

**G1 also runs (duplicate coverage):** `cargo test -p arks-object --test g001-loose-object-interop_oracle` inside `run-g001-loose-object-interop.sh`.

**Adversarial G0 (bundled in crate units, not separate IDs):** wrong digest/hex lengths; mixed-case hex; missing NUL / bad size / unknown kind; corrupt zlib; OID mismatch on corrupt loose; write_loose idempotency; type-token OID divergence.

Legend: **G0** = `verify/scripts/run-g000-workspace-lib-tests.sh` (`cargo test --workspace` in-image); **G1** = `verify/scripts/run-g001-loose-object-interop.sh` (`verify/gates/g001-loose-object-interop` oracle + workspace `g001-loose-object-interop_oracle`).
