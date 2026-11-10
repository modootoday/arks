# M4 clean-room usage-condition matrix (G0 / G4 / G5 + G1/G2/G10 regression)

Roadmap **v0.3.1** M4 (Pack + protocol client libs / Phase P + gix-exit). Authoritative venue: Docker clean-room under `verify/` per [[sot:root/product-verification-clean-room]]. Image pin: `verify/images.lock.json` (`arks-verify-m1:0.1.0`). Depends on M3 [[evidence:root/20261108191033]].

**Authoritative runs:** `verify/docker-compose.yml` services `g000-workspace-lib-tests`, `g004-pack-idx-roundtrip` (G4 embeds G5 + G2/G10/G1 regression), `g005-protocol-offline-fixtures` (standalone). Stock Git in-image only for G4. Unit SoTs: [[sot:root/arks-pack]], [[sot:root/arks-protocol]].

| ID | Condition | Gate | Automated | Entry |
|----|-----------|------|-----------|-------|
| M4-C01 | arks undeltified pack → `git index-pack --strict` + idx offset/CRC parity | G4 | yes | `verify/gates/g004-pack-idx-roundtrip` `arks_undeltified_pack_accepted_by_git_index_pack` |
| M4-C02 | arks pack → `git verify-pack -v` | G4 | yes | (same test) |
| M4-C03 | `git pack-objects` → arks `DecodedPack` OID set match | G4 | yes | `verify/gates/g004-pack-idx-roundtrip` `git_pack_objects_decoded_by_arks` |
| M4-C04 | `git pack-objects` idx → arks `PackIndex::lookup` | G4 | yes | (same test) |
| M4-C05 | arks pack+idx self round-trip | G4 | yes | `verify/gates/g004-pack-idx-roundtrip` `arks_roundtrip_matches_own_oids` |
| M4-C06 | pkt-line data/flush/delim/response-end offline | G5 | yes | `verify/gates/g005-protocol-offline-fixtures` `pkt_line_specials_and_data_offline` |
| M4-C07 | capability advertise/parse offline | G5 | yes | `verify/gates/g005-protocol-offline-fixtures` `capability_advertise_parse_offline` |
| M4-C08 | v1 advertisement fragment offline | G5 | yes | `verify/gates/g005-protocol-offline-fixtures` `v1_advertisement_fragment_offline` |
| M4-C09 | v2 capability advertisement offline | G5 | yes | `verify/gates/g005-protocol-offline-fixtures` `v2_capability_advertisement_offline` |
| M4-C10 | Workspace library tests (incl. pack/protocol) | G0 | yes | `cargo test --workspace` via `run-g000-workspace-lib-tests.sh` |
| M4-C11 | M3 G2 refs/index regression | G2 | yes | `verify/gates/g002-refs-index-roundtrip` via `run-g004-pack-idx-roundtrip.sh` |
| M4-C12 | M2 G10 facade regression | G10 | yes | `verify/gates/g010-store-facade` via `run-g004-pack-idx-roundtrip.sh` |
| M4-C13 | M1 G1 oracle regression | G1 | yes | `arks-object` + `verify/gates/g001-loose-object-interop` via `run-g004-pack-idx-roundtrip.sh` |
| M4-C14 | `cargo metadata` zero `gix`/gitoxide edges | gix-exit | yes | documented in evidence receipt |
| M4-C15 | Delta **encode** / pack v3 / MIDX / `.rev` | — | no | Out of M4 claimed subset |
| M4-C16 | Live remotes / upload-pack client | — | no | Deferred to M6 |
| M4-C17 | CLI porcelain / full G3 | — | no | Deferred to M5 |
| M4-C18 | CELIR / lease L1+ | — | no | Out of M4 |

Legend: **G4** = pack/idx vs stock `git`; **G5** = offline protocol fixtures; regressions embed G2+G10+G1.

Host-only green does **not** carry M4 exit.
