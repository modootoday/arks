# G28 — live private arks journeys

SoT: `.spec/sot/g028-live-private-arks-live-private-arks-journeys.sot.md`
Plan: `.spec/plan/20261109212020-g028-live-private-arks-live-private-arks-journeys.plan.md`
Entrypoint: `verify/scripts/run-g028-live-private-arks.sh` / compose `g028-live-private-arks`

**Honesty:** functional journey coverage on throwaway **private** GitHub with arks — **not** scale/contention, **not** G16 timing compare, **not** Checks/merge-queue product claim.

**Opt-in (live):** `ARKS_G28_LIVE=1` + token + `ARKS_G28_REPO_URL`. Default without secrets = Refuse-closed (ADR G8 affinity).

**Offline clones:** `docker compose -f verify/docker-compose.yml run --rm g028-offline-clones` closes **J-CLONE-LOCAL-01** / **J-CLONE-PATH-01** (no live forge).

| ID | Steps (sketch) | Live? | Gate |
|----|----------------|-------|------|
| J-LOCAL-01 | add/commit/status/log + push | yes | G28 |
| J-REMOTE-CLONE-01 | `arks clone` HTTPS private | yes | G28 |
| J-BRANCH-01 | branch/checkout/switch + push | yes | G28 |
| J-MERGE-FF-01 | merge --ff-only + push | yes | G28 |
| J-PUSH-FETCH-01 | push; fetch tip | yes | G28 |
| J-PULL-01 | pull into second worktree | yes | G28 |
| J-TAG-01 | tag -a -m; push tag | yes | G28 |
| J-STASH-01 | stash push/list/pop | yes (local) | G28 |
| J-DIFF-01 | diff / diff --cached | yes (local) | G28 |
| J-RM-MV-01 | mv/rm + commit + push | yes | G28 |
| J-CHERRY-01 | cherry-pick + push | yes | G28 |
| J-REBASE-01 | rebase + push | yes | G28 |
| J-REVERT-01 | revert + push | yes | G28 |
| J-REFLOG-01 | reflog show | yes (local) | G28 |
| J-BUNDLE-01 | bundle create/verify | yes (local) | G28 |
| J-ARCHIVE-01 | archive tar | yes (local) | G28 |
| J-REPACK-01 | repack -ad | yes (local) | G28 |
| J-WORKTREE-01 | worktree add/list/remove | yes (local) | G28 |
| J-SUBMODULE-01 | submodule add HTTPS / status | yes when feasible | G28 |
| J-PR-GH-01 | push topic; open PR (gh/API) | yes | G28 |
| J-ALIAS-01 | PATH shim short local slice | optional | G28 |
| J-CLONE-LOCAL-01 | smart-HTTP local (G16-aligned seed) | offline | **G28 offline** (`g028-offline-clones`) — PASS [[evidence:root/20261109213400]] |
| J-CLONE-PATH-01 | path/file:// clone (G16-aligned seed) | offline | **G28 offline** (`g028-offline-clones`) — PASS [[evidence:root/20261109213400]] |
