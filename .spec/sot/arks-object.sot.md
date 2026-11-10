---
id: sot:root/arks-object
title: arks-object unit
status: active
created: 2026-11-08T09:18:28Z
updated: 2026-11-10T11:08:00Z
tags: [m1, m5, unit, objects, loose, tree, commit, ring-1, gpgsig, fw-gpgsig, r4, s2, openpgp, x509]
verify: manual
rel:
  governs: [crates/arks-object]
  depends_on:
    - sot:root/core-object-model
    - sot:root/crate-architecture
    - sot:root/git-compat-surface
    - sot:root/product-verification-clean-room
    - sot:root/program-roadmap
    - sot:root/interop-test-matrix
    - sot:root/arks-hash
    - sot:root/arks-oid
    - research:root/20261108071913
    - research:root/20261110105436
    - plan:root/20261108090646
    - plan:root/20261108090650
    - adr:root/20261110110450
  evidence:
    - evidence:root/20261108185319
    - evidence:root/20261110170000
    - evidence:root/20261110073800
    - evidence:root/20261110110800
---

# arks-object unit

Version **0.1.5**. Unit SoT for Ring 1 loose object encode/decode ([[plan:root/20261108090646]], Wave-1 W1.6) plus **M5** tree/commit payload grammar for Wave L commit/log/show. **FW-GPGSIG:** read + **SSH / OpenPGP / X.509 create/verify** + **signed tags** claimed subset (residual **R4** / symmetry **S2**). Unlock: [[adr:root/20261110110450]].

## Rule

1. **Claimed kinds (M1):** `blob`, `tree`, `commit`, `tag` only. Unknown type tokens MUST refuse ([[research:root/20261108071913]], gitformat-loose).
2. **Uncompressed form:** `type SP size NUL` + payload. OID is the hash of those uncompressed bytes via [[sot:root/arks-hash]] ([[sot:root/core-object-model]]).
3. **Loose file:** zlib-compress the uncompressed stream; path under `objects/aa/bbbb…` per gitformat-loose / gitrepository-layout.
4. **Interop format for G1:** SHA-1 object format.
5. **Tree/commit payloads (M5):** encode/decode `Tree` / `TreeEntry` / `TreeMode` and `Commit` / `Signature` per stock Git accepted encodings. Cite [[research:root/20261108071913]].
6. **Optional commit headers (FW-GPGSIG read):** Decode MUST accept optional headers including multiline `gpgsig` (and `encoding` / `mergetag`) using SP-prefixed continuation rules from [gitformat-signature](https://git-scm.com/docs/gitformat-signature). Decode MUST capture `gpgsig` armor text (without SP prefixes) when present; other optional headers remain skip-only. Unsigned encode MAY omit `gpgsig`.
7. **Commit signature create/verify (FW-GPGSIG):** When CLI claims `commit -S` / `verify-commit` ([[sot:root/arks]], [git-commit](https://git-scm.com/docs/git-commit), [git-verify-commit](https://git-scm.com/docs/git-verify-commit)):
   - **Signing payload** is the commit object content **without** the `gpgsig` header.
   - **Embedding:** multiline `gpgsig` after `committer` with SP-prefixed continuations.
   - **Format** follows `gpg.format` ([git-config](https://git-scm.com/docs/git-config)):
     - `ssh`: armor `BEGIN SSH SIGNATURE`; spawn `gpg.ssh.program` (default `ssh-keygen`) `-Y sign` / `-Y verify`.
     - `openpgp` / default: armor `BEGIN PGP SIGNATURE` (or `MESSAGE`); spawn `gpg.openpgp.program` / `gpg.program` (default `gpg`) `-bsau` / `--verify`.
     - `x509`: armor `BEGIN SIGNED MESSAGE`; spawn `gpg.x509.program` (default `gpgsm`) `-bsau` / `--verify`.
   - **Crypto:** MUST NOT invent signature bytes in-process.
   - **Still unclaimed:** `key::` literal SSH keys.
8. **Signed tags (R4 / S2):** `tag -s` / `-u <key>` create annotated tag objects with the detached signature **appended** after the unsigned tag payload ([gitformat-signature](https://git-scm.com/docs/gitformat-signature) tag signatures; [git-tag](https://git-scm.com/docs/git-tag)). `verify-tag` verifies using the same format dispatch as commits. Format follows `gpg.format`.
9. **No product-path `git` subprocess** and **no `gix`** ([[adr:root/20261108083507]]). Spawning configured `ssh-keygen` / `gpg` / `gpgsm` is allowed.
10. **Out of scope:** pack writers as product claims; forge signature UX.

Primary: https://git-scm.com/docs/gitformat-loose ; https://git-scm.com/docs/gitformat-signature

## Why

SSH signing was Claimed via OpenSSH (G37). Symmetry **S2** closes OpenPGP / X.509 / signed-tag refuse gaps without inventing crypto — spawn Git’s configured programs. Evidence venue is Docker clean-room (**G38**), not host Windows GnuPG keygen loops.

## Verification

1. Host `cargo test -p arks-object` — advisory.
2. Clean-room G0/G1 as before.
3. FW-GPGSIG SSH create/verify: G37 evidence [[evidence:root/20261110073800]].
4. FW-GPGSIG OpenPGP / X.509 / `tag -s` / `verify-tag`: clean-room **G38** (`verify/matrices/matrix-g038-gpgsig-openpgp.md`) — Claimed only with carrying evidence. Host gpg abort loops MUST NOT invent PASS.
5. `spec check` clean for this SoT.
