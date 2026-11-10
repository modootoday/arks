---
id: adr:root/20261110110450
title: Reopen R4 — OpenPGP / X.509 / tag -s / verify-tag
status: active
created: 2026-11-10T11:04:50Z
updated: 2026-11-10T11:08:00Z
tags: [r4, s2, openpgp, x509, tag-s, verify-tag, gpgsig, symmetry]
rel:
  depends_on:
    - sot:root/arks-object
    - sot:root/production-residual-register
    - research:root/20261110105436
    - sot:root/interop-test-matrix
    - sot:root/cli-surface
  supersedes: []
  evidence: [evidence:root/20261110110800]
---

# Reopen R4 — OpenPGP / X.509 / `tag -s` / `verify-tag`

## Context

Residual **R4** / historical ladder **L7** closed OpenPGP / X.509 / signed tags as honest **N/A** (CLI refuse). Symmetry ladder [[research:root/20261110105436]] **S2** reopens the crypto gap. SSH commit create/verify remains Claimed (**G37**). Host GnuPG keygen on Windows MSYS is hang-prone — evidence MUST prefer Docker clean-room (`arks-verify-m1`) which already ships `gpg` / `gpgsm`.

## Decision

Amend [[sot:root/arks-object]] + CLI to claim an additive **FW-GPGSIG expand**:

1. **OpenPGP** (`gpg.format=openpgp` / default): `commit -S` / `verify-commit` via configured `gpg.program` / `gpg.openpgp.program` (default `gpg`) using Git’s interface (`-bsau` sign; `--verify` verify). Cite [git-config gpg.*](https://git-scm.com/docs/git-config), [gitformat-signature](https://git-scm.com/docs/gitformat-signature).
2. **X.509** (`gpg.format=x509`): same create/verify surfaces via `gpg.x509.program` (default `gpgsm`) with `-----BEGIN SIGNED MESSAGE-----` armor.
3. **Signed tags:** `tag -s` / `-u` create annotated tags with signature **appended** to the unsigned tag payload; `verify-tag` verifies. Format follows `gpg.format` (OpenPGP / X.509 / SSH).
4. **Crypto honesty:** MUST spawn the configured program — MUST NOT invent signature bytes in-process.
5. **Evidence venue:** clean-room compose **G38** (OpenPGP primary + stock-Git verify). Host Windows gpg keygen is **not** the carrying bar.

## Non-claims

- Forge web UI signature badges / Checks.
- `key::` literal SSH keys (still unclaimed).
- Inventing PASS from host gpg hang/abort loops.

## Consequences

- Residual **R4** → **Claimed** only with G38 (or equivalent) receipt.
- G37 remains SSH commit matrix; G37 openpgp-refuse row is superseded by G38 Claimed OpenPGP.
- Symmetry **S2** Done when SoT + wire + evidence agree; next **S3**.
