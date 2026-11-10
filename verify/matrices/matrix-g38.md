# G38 clean-room matrix — FW-GPGSIG OpenPGP / X.509 / tag -s (R4 / S2)

Unlock: [[adr:root/20261110110450]], [[sot:root/arks-object]] v0.1.5. SSH commit remains **G37**.

Primary: [gitformat-signature](https://git-scm.com/docs/gitformat-signature), [git-commit `-S`](https://git-scm.com/docs/git-commit), [git-verify-commit](https://git-scm.com/docs/git-verify-commit), [git-tag `-s`](https://git-scm.com/docs/git-tag), [git-verify-tag](https://git-scm.com/docs/git-verify-tag), [git-config `gpg.format`](https://git-scm.com/docs/git-config).

| ID | Scenario | Gate | Automated | Expect |
|----|----------|------|-----------|--------|
| G38-C01 | Ephemeral OpenPGP key; `arks commit -S` embeds PGP `gpgsig` | G38 | yes | BEGIN PGP SIGNATURE |
| G38-C02 | `arks verify-commit HEAD` | G38 | yes | exit 0 |
| G38-C03 | Stock `git verify-commit HEAD` | G38 | yes | exit 0 |
| G38-C04 | `arks tag -s -m …` + `arks verify-tag` | G38 | yes | armor appended; exit 0 |
| G38-C05 | Stock `git verify-tag` | G38 | yes | exit 0 |
| G38-C06 | X.509 via `gpg.x509.program` mock roundtrip (arks create/verify) | G38 | yes | BEGIN SIGNED MESSAGE |
| G38-C07 | gix-exit scan | G38 | yes | clean |

Host Windows GnuPG keygen is **not** the carrying bar (hang/abort risk). Clean-room `gpg` / `gpgsm` on `arks-verify-m1` only.
