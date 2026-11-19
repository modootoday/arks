# Matrix G42 — cloud agent drop-in

SoT: `.spec/sot/cloud-agent-dropin.sot.md` §V · Plan: `.spec/plan/20261119210000-cloud-dropin-remaining-rungs.plan.md` · Entry: `verify/scripts/run-g042-cloud-dropin.sh` (compose `g042-cloud-dropin`).

Claimed only from a Docker clean-room receipt (`verify/gates/g042-cloud-dropin/out/receipt.json`, pin git 2.39.5). Host runs are advisory.

| Row | Must | Check |
|-----|------|-------|
| T-CLONE-VIA-PROXY | yes | `arks clone https://arks-g42.test/repo.git` with `GIT_SSL_CAINFO` + `HTTPS_PROXY`; host resolves only through the proxy |
| T-PROXY-USED | yes | proxy log shows `CONNECT arks-g42.test:443` |
| C-IDENTITY-CONFIG | yes | `arks config user.name` reads the global config file |
| W-BRANCH | yes | `arks checkout -b g42-topic` |
| W-COMMIT-AM | yes | `arks commit -am` |
| S-GPGSIG | yes | commit carries `gpgsig` (`commit.gpgsign`, `gpg.format=ssh`) |
| S-VERIFY-COMMIT | yes | stock `git verify-commit` with `allowedSignersFile` |
| C-AUTHOR | yes | author from config, not a placeholder |
| W-PUSH-U | yes | `arks push -u origin g42-topic` through the proxy |
| W-FETCH-HEAD | yes | `arks fetch origin main` writes `FETCH_HEAD` |
| W-MERGE | yes | `arks merge --no-edit FETCH_HEAD` |
| O-PEER-CLONE / FSCK / VERIFY / TIP | yes | stock git re-clone, `fsck`, `verify-commit`, tip equality |
| corpus | no (recorded) | `run-cli-flag-corpus-compare.sh` agent tier A SAME/DIFF counts vs pin git |

## Receipts

| Date | Venue | Result |
|------|-------|--------|
| 2026-11-19 | host advisory (git 2.43.0, no Docker daemon) | 15/15 must PASS; corpus 78/147 SAME — **not** a clean-room receipt |
| 2026-11-19 | host advisory (git 2.43.0, no Docker daemon), after C7a | 15/15 must PASS; corpus 146/147 SAME — **not** a clean-room receipt |
