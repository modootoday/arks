# G29 clean-room matrix — server dumb HTTP (M25 / FW-SRV-DUMB)

Unlock: [[adr:root/20261109220230]]. Unit SoT: [[sot:root/arks-server]]. Primary: [gitprotocol-http](https://git-scm.com/docs/gitprotocol-http) dumb section; [git-update-server-info](https://git-scm.com/docs/git-update-server-info). Smart HTTP remains required (G15); dumb is **additive**.

| ID | Scenario | Gate | Automated | Expect |
|----|----------|------|-----------|--------|
| G29-C01 | `update-server-info` writes `info/refs` + `objects/info/packs` | G29 | yes | files present; refs TAB format |
| G29-C02 | Stock `git clone` via arks `--dumb-only` HTTP | G29 | yes | objects/refs match |
| G29-C03 | Dumb GET loose object path | G29 | yes | 200 + zlib loose bytes |
| G29-C04 | zero gix / zero celir on arks-server | G29 | yes | metadata scan |

Host-only green does **not** carry G29 Claimed.
