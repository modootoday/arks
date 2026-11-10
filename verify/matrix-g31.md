# G31 clean-room matrix — server protocol v2 (M25 / FW-SRV-V2)

Unlock: [[adr:root/20261109220230]]. Primary: [gitprotocol-v2](https://git-scm.com/docs/gitprotocol-v2). Claimed: `ls-refs` + `fetch` subset over smart HTTP with `Git-Protocol: version=2`.

| ID | Scenario | Gate | Automated | Expect |
|----|----------|------|-----------|--------|
| G31-C01 | info/refs with Git-Protocol version=2 → `version 2` ad | G31 | yes | ls-refs + fetch advertised |
| G31-C02 | Stock `git -c protocol.version=2 clone` vs arks smart HTTP | G31 | yes | objects/refs match |
| G31-C03 | No invented caps in v2 ad | G31 | yes | only claimed keys |
| G31-C04 | v1 still works without version=2 | G31 | yes | G15-style clone |

Host-only green does **not** carry G31 Claimed.
