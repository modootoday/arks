# G32 clean-room matrix — `git://` daemon (M25 / FW-SRV-GIT)

Unlock: [[adr:root/20261109220230]]. Primary: [git-daemon](https://git-scm.com/docs/git-daemon), [gitprotocol-pack](https://git-scm.com/docs/gitprotocol-pack).

| ID | Scenario | Gate | Automated | Expect |
|----|----------|------|-----------|--------|
| G32-C01 | Stock `git clone git://127.0.0.1:port/repo` vs `arks daemon --export-all` | G32 | yes | tip matches |
| G32-C02 | Without export marker / without `--export-all` → refuse | G32 | yes | clone fails |
| G32-C03 | upload-pack only by default (receive disabled) | G32 | yes | push refused |
| G32-C04 | zero gix / zero celir | G32 | yes | metadata scan |

Host-only green does **not** carry G32 Claimed.
