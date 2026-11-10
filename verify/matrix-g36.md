# G36 clean-room matrix — optional HTTP Basic (FW-AUTH-PROXY)

Unlock: [[adr:root/20261109224236]]. Unit SoT: [[sot:root/arks-server]]. Primary: [gitprotocol-http](https://git-scm.com/docs/gitprotocol-http) (auth as transport); RFC 7617 Basic (summarize — Base64 `userid:password`, `WWW-Authenticate: Basic realm="arks"`). G15 unauthenticated smart HTTP remains Claimed. Forge OAuth still Refuse. Gates **G33–G35** are reserved elsewhere — do not reuse.

| ID | Scenario | Gate | Automated | Expect |
|----|----------|------|-----------|--------|
| G36-C01 | Missing Authorization → 401 + `WWW-Authenticate: Basic realm="arks"` | G36 | yes | challenge headers |
| G36-C02 | Stock `git clone` with URL userinfo against `--basic-auth` | G36 | yes | objects/refs match |
| G36-C03 | Wrong password refused; correct credential via helper/env succeeds | G36 | yes | fail then pass |
| G36-C04 | Unset Basic still allows unauthenticated loopback (G15 shape) | G36 | yes | clone without creds |
| G36-C05 | zero gix / zero celir / no lease on arks-server | G36 | yes | metadata scan |

Host-only green does **not** carry G36 Claimed.
