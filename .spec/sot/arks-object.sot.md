---
id: sot:root/arks-object
title: arks-object unit
status: active
created: 2026-11-08T09:18:28Z
updated: 2026-11-10T08:00:00Z
tags: [m1, m5, unit, objects, loose, tree, commit, ring-1, gpgsig, fw-gpgsig]
verify: manual
rel:
  governs: [crates/arks-object]
  depends_on: [sot:root/core-object-model, sot:root/crate-architecture, sot:root/git-compat-surface, sot:root/product-verification-clean-room, sot:root/program-roadmap, sot:root/interop-test-matrix, sot:root/arks-hash, sot:root/arks-oid, research:root/20261108071913, plan:root/20261108090646, plan:root/20261108090650]
  evidence: [evidence:root/20261108185319, evidence:root/20261110170000, evidence:root/20261110073800]
---

# arks-object unit

Version 0.1.4. Unit SoT for Ring 1 loose object encode/decode ([[plan:root/20261108090646]], Wave-1 W1.6) plus **M5** tree/commit payload grammar for Wave L commit/log/show. **FW-GPGSIG:** read + **SSH create/verify** claimed subset; OpenPGP/`gpg` and X.509 remain unclaimed.

## Rule

1. **Claimed kinds (M1):** `blob`, `tree`, `commit`, `tag` only. Unknown type tokens MUST refuse ([[research:root/20261108071913]], gitformat-loose).
2. **Uncompressed form:** `type SP size NUL` + payload. OID is the hash of those uncompressed bytes via [[sot:root/arks-hash]] ([[sot:root/core-object-model]]).
3. **Loose file:** zlib-compress the uncompressed stream; path under `objects/aa/bbbb…` per gitformat-loose / gitrepository-layout.
4. **Interop format for G1:** SHA-1 object format.
5. **Tree/commit payloads (M5):** encode/decode `Tree` / `TreeEntry` / `TreeMode` and `Commit` / `Signature` per stock Git accepted encodings. Cite [[research:root/20261108071913]].
6. **Optional commit headers (FW-GPGSIG read):** Decode MUST accept optional headers including multiline `gpgsig` (and `encoding` / `mergetag`) using SP-prefixed continuation rules from [gitformat-signature](https://git-scm.com/docs/gitformat-signature). Decode MUST capture `gpgsig` armor text (without SP prefixes) when present; other optional headers remain skip-only. Unsigned encode MAY omit `gpgsig`.
7. **Commit signature create/verify (FW-GPGSIG SSH subset):** When CLI claims `commit -S` / `verify-commit` with `gpg.format=ssh` ([[sot:root/arks]], [git-commit](https://git-scm.com/docs/git-commit), [git-verify-commit](https://git-scm.com/docs/git-verify-commit)):
   - **Signing payload** is the commit object content **without** the `gpgsig` header.
   - **Embedding:** multiline `gpgsig` after `committer` with SP-prefixed continuations.
   - **Armor:** `-----BEGIN SSH SIGNATURE-----` … `-----END SSH SIGNATURE-----`.
   - **Crypto:** spawn configured `gpg.ssh.program` (default `ssh-keygen`) with `-Y sign` / `-Y verify` — MUST NOT invent signature bytes in-process. Cite [git-config `gpg.format` / `gpg.ssh.*`](https://git-scm.com/docs/git-config).
   - **Unclaimed:** `openpgp`/`gpg` and `x509`; tag `-s` / `verify-tag`; `key::` literal keys.
8. **No product-path `git` subprocess** and **no `gix`** ([[adr:root/20261108083507]]). Spawning pinned `ssh-keygen` is allowed.
9. **Out of scope:** pack writers as product claims; OpenPGP/X.509 create/verify; signed tags.

Primary: https://git-scm.com/docs/gitformat-loose ; https://git-scm.com/docs/gitformat-signature

## Why

SSH signing is claimed via OpenSSH already pinned in the clean-room image.

## Verification

1. Host `cargo test -p arks-object` — advisory.
2. Clean-room G0/G1 as before.
3. FW-GPGSIG SSH create/verify: G37 evidence [[evidence:root/20261110073800]].
4. `spec check` clean for this SoT.
