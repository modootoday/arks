---
id: evidence:root/20261110110800
title: R4 OpenPGP X.509 tag-s Claimed (G38 clean-room)
status: active
created: 2026-11-10T11:08:00Z
updated: 2026-11-10T11:12:00Z
tags: [r4, s2, openpgp, x509, tag-s, verify-tag, g038-gpgsig-openpgp, clean-room, claimed, symmetry]
rel:
  depends_on:
    - adr:root/20261110110450
    - sot:root/arks-object
    - research:root/20261110105436
    - sot:root/production-residual-register
  evidence: []
files: []
---

# R4 — OpenPGP / X.509 / `tag -s` Claimed (G38 clean-room)

## Claimed subset (honest)

| In | Out |
|----|-----|
| `gpg.format=openpgp`: `commit -S` / `verify-commit` via `gpg -bsau` / `--verify` | Inventing signature bytes in-process |
| `tag -s` / `-u` + `verify-tag` (OpenPGP armor appended) | Forge signature UX / Checks badges |
| Stock `git verify-commit` / `git verify-tag` vs arks OpenPGP objects | Host Windows GnuPG keygen as carrying bar |
| `gpg.format=x509`: spawn `gpg.x509.program` (default `gpgsm`); arks create/verify roundtrip | Stock-Git X.509 PKI peer as Verified (mock proves wire) |
| SSH commit path unchanged (**G37**) | `key::` literal SSH keys |

Primary: [gitformat-signature](https://git-scm.com/docs/gitformat-signature), [git-commit `-S`](https://git-scm.com/docs/git-commit), [git-tag `-s`](https://git-scm.com/docs/git-tag), [git-verify-tag](https://git-scm.com/docs/git-verify-tag), [git-config `gpg.format`](https://git-scm.com/docs/git-config). Unlock: [[adr:root/20261110110450]]. Symmetry ladder: [[research:root/20261110105436]] **S2**.

## Clean-room receipt (Claimed bar)

| Check | Outcome |
|-------|---------|
| `docker compose -f verify/docker-compose.yml run --rm --no-deps g038-gpgsig-openpgp` | **PASS** — `G38: PASS` (tip worktree; Git **2.39.5**, gpg **2.2.40**, rustc **1.99.1**) |
| `g038-gpgsig-openpgp_openpgp_commit_and_tag_stock_roundtrip` | PASS — ephemeral OpenPGP in-image; arks + stock `git verify-commit` / `verify-tag` |
| `g038-gpgsig-openpgp_x509_mock_program_roundtrip` | PASS — configured `gpg.x509.program` mock; arks create/verify |
| Venue | `arks-verify-m1:0.1.0` (image ships `gpg` / `gpgsm`) |

Host Windows MSYS gpg keygen / cargo stampede exit `4294967295` is **not** evidence. Host unit tests skip OpenPGP keygen unless `ARKS_HOST_GPG=1`.

## Non-claims

- Live forge / G8 latency PASS.
- G19 remasure.
- Treating X.509 mock as a stock `gpgsm` certificate authority Verified peer.
