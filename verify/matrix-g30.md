# G30 clean-room matrix — server SSH command-mode (M25 / FW-SRV-SSH)

Unlock: [[adr:root/20261109220230]]. Primary: [gitprotocol-pack](https://git-scm.com/docs/gitprotocol-pack), [git-upload-pack](https://git-scm.com/docs/git-upload-pack), [git-receive-pack](https://git-scm.com/docs/git-receive-pack). No product SSHD — stock Git `--upload-pack` / `--receive-pack` invokes `arks` duplex stdio (same wire as SSH ForceCommand).

| ID | Scenario | Gate | Automated | Expect |
|----|----------|------|-----------|--------|
| G30-C01 | Stock `git clone --upload-pack=arks\ upload-pack` local | G30 | yes | tip matches |
| G30-C02 | Stock `git fetch` via upload-pack duplex | G30 | yes | tip advances |
| G30-C03 | Stock `git push --receive-pack=arks\ receive-pack` | G30 | yes | report-status ok |
| G30-C04 | Pack ad has no HTTP `# service=` prefix | G30 | yes | honesty |

Host-only green does **not** carry G30 Claimed.
