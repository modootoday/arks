# G37 clean-room matrix — FW-GPGSIG SSH create/verify

Primary: [gitformat-signature](https://git-scm.com/docs/gitformat-signature), [git-commit `-S`](https://git-scm.com/docs/git-commit), [git-verify-commit](https://git-scm.com/docs/git-verify-commit), [git-config `gpg.format` / `gpg.ssh.*`](https://git-scm.com/docs/git-config). Owning SoT: [[sot:root/arks-object]] v0.1.4. Evidence: [[evidence:root/20261110073800]].

| ID | Scenario | Gate | Automated | Expect |
|----|----------|------|-----------|--------|
| G37-C01 | Ephemeral ed25519; `arks commit -S` embeds SSH `gpgsig` | G37 | yes | BEGIN/END SSH SIGNATURE |
| G37-C02 | `arks verify-commit HEAD` | G37 | yes | exit 0 |
| G37-C03 | Stock `git cat-file -p` shows gpgsig | G37 | yes | subject + armor |
| G37-C04 | Stock `git verify-commit` | G37 | yes | exit 0 |
| G37-C05 | `gpg.format=openpgp` refuse | G37 | yes | non-zero |
| G37-C06 | gix-exit scan | G37 | yes | clean |

Host-only green does **not** carry G37 Claimed. Ephemeral keys only. OpenSSH already pinned in `arks-verify-m1`.
