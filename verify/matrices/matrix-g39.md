# G39 clean-room matrix — R3 `blob:none` symmetry twin (S6)

Unlock: [[adr:root/20261110105547]], [[evidence:root/20261110105855]] (host wire), [[sot:root/arks-remote]] v0.5.4.
Primary: [partial-clone](https://git-scm.com/docs/partial-clone), [git-clone `--filter`](https://git-scm.com/docs/git-clone).

| ID | Scenario | Gate | Automated | Expect |
|----|----------|------|-----------|--------|
| G39-C01 | `arks clone --filter=blob:none` vs stock `git http-backend` (allowFilter) | G39 | yes | HEAD worktree + `extensions.partialClone` + `.promisor` |
| G39-C02 | Refuse unclaimed `--filter=tree:0` | G39 | yes | non-zero refuse honesty |
| G39-C03 | gix-exit scan | G39 | yes | clean |

**Non-claims:** `tree:0`, `blob:limit=*`, multi-promisor product, inventing filter caps.
