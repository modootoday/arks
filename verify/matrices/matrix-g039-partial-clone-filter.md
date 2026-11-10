# G39 clean-room matrix — R3/O2 partial-clone filter symmetry twin

Unlock: [[adr:root/20261110105547]] (`blob:none`), [[adr:root/20261110210000]] (`tree:0`), [[sot:root/arks-remote]] v0.5.5.
Primary: [partial-clone](https://git-scm.com/docs/partial-clone), [git-clone `--filter`](https://git-scm.com/docs/git-clone).

| ID | Scenario | Gate | Automated | Expect |
|----|----------|------|-----------|--------|
| G39-C01 | `arks clone --filter=blob:none` vs stock `git http-backend` (allowFilter) | G39 | yes | HEAD worktree + `extensions.partialClone` + `.promisor` |
| G39-C02 | `arks clone --filter=tree:0` vs stock peer (nested tree) | G39 | yes | HEAD worktree incl. nested path + `partialClone` |
| G39-C03 | Refuse unclaimed `--filter=blob:limit=1k` | G39 | yes | non-zero refuse honesty |
| G39-C04 | gix-exit scan | G39 | yes | clean |

**Non-claims:** `blob:limit=*`, sparse OID filters, multi-promisor product, inventing filter caps.
