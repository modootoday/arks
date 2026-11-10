# G23 CELIR dual-path Git verified matrix

**Authoritative runs:** `verify/docker-compose.yml` service `g23` (`verify/scripts/run-g23.sh`).
Host runs are advisory only.

**Owning SoT:** `.spec/sot/celir-dual-path-git-verified.sot.md`
**ADR Done:** `.spec/adr/20261109204916-celir-dual-path-git-verified-done.adr.md`
**Plan:** `.spec/plan/20261109204933-m24-g23-celir-dual-path-git-verified.plan.md`

**Honesty:** FS `.git` sole ObjectStore + CELIR index sidecar. Not CELIR as ObjectStore. lease ≠ skip locks.

## Cases

| Case | Gate | How |
|------|------|-----|
| G23-C01 | g23 | arks init/add/commit/status/log; stock `git` log/cat-file/rev-parse OK |
| G23-C02 | g23 | Same worktree: celir-index rebuild/publish/open/query; lookup matches |
| G23-C03 | g23 | ObjectStore filesystem-only; default graph CELIR-free |
| G23-C04 | g23 | Sole-runner lock + discrete-circuit sibling mount |

## Done for Verified

Docker `g23` PASS + evidence stamp; residual/roadmap may say **CELIR dual-path Git verified**.
