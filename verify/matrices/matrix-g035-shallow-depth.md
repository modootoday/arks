# G35 clean-room matrix — client shallow `--depth` (FW-G9 / G9 Claimed subset)

Unlock: [[sot:root/arks-remote]] v0.4.0 Claimed `--depth N` subset. Historical ladder id **G9**; compose service **G35** (coordination: G33=CRED, G34=SHARED-IDX).

Primary:
- [git-clone `--depth`](https://git-scm.com/docs/git-clone#Documentation/git-clone.txt---depthltdepthgt)
- [gitprotocol-capabilities](https://git-scm.com/docs/gitprotocol-capabilities) `shallow`
- [gitprotocol-pack](https://git-scm.com/docs/gitprotocol-pack) deepen / shallow-update
- [partial-clone](https://git-scm.com/docs/partial-clone) — **not claimed** (filter / promisor N/A)

| ID | Scenario | Gate | Automated | Expect |
|----|----------|------|-----------|--------|
| G35-C01 | `arks clone --depth 1` via loopback smart HTTP vs stock `git http-backend` | G35 | yes | tip checkout; `.git/shallow` lists tip; `rev-list --count HEAD` = 1 |
| G35-C02 | `arks fetch --depth 1` into empty tracking clone path (or deepen honesty) | G35 | yes | shallow file present; no invented `filter` cap |
| G35-C03 | Local path / `file://` `clone --depth` refuses | G35 | yes | non-zero; no dest mutation beyond refuse |
| G35-C04 | zero gix / zero celir on `arks-remote` | G35 | yes | metadata scan |

**Non-claims:** `blob:none`, `tree:0`, promisor remotes, `extensions.partialClone`, `deepen-since` / `deepen-not` / `--unshallow`.

Host-only green does **not** carry G35 Claimed / G9 product-verified.
