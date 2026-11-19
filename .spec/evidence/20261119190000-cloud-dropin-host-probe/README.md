---
id: evidence:root/20261119190000-cloud-dropin-host-probe
title: Cloud agent drop-in host probe (advisory)
created: 2026-11-19T19:00:00Z
tags: [cloud, dropin, host-advisory, proxy, signing, corpus]
rel:
  depends_on:
    - sot:root/cloud-agent-dropin
    - plan:root/20261119180000
---

# Cloud agent drop-in host probe (advisory)

**Class: HOST ADVISORY.** Not a Docker clean-room receipt; MUST NOT be cited as Claimed or PASS
for [[sot:root/cloud-agent-dropin]] §V. The sandbox had no Docker daemon, so no clean-room run was possible.

Entry: `verify/scripts/run-cloud-dropin-host-probe.sh target/release/arks https://github.com/modootoday/arks <out>`
inside a hosted agent sandbox (TLS-intercepting egress proxy, `GIT_SSL_CAINFO` bundle,
global `~/.gitconfig` with `commit.gpgsign=true`, `gpg.format=ssh`, sandbox `gpg.ssh.program`).
Peer oracle: stock git 2.43.0 on the same clone (not the 2.39.5 pin).

## Result (`receipt.json`)

| Class | Count |
|-------|-------|
| must PASS | 12 / 12 (clone, fsck, ls-remote via proxy; `commit -am` with config identity; `gpgsig` present; fsck after commit; `checkout -B`; `fetch <branch>` → `FETCH_HEAD`; `merge --no-edit FETCH_HEAD`; `push --dry-run`) |
| compare SAME | 19 / 19 (log/format/range/path, rev-parse `~`/`^`, show `--stat`/`--name-only`, diff range `--stat`/`--name-status`, `branch --show-current`, `remote -v`, `config`, `status -sb`/`--porcelain`, `diff`, `diff HEAD -- <path>`, `diff --stat`) |

## Supplementary sweeps (same host, same clone)

| Sweep | Result |
|-------|--------|
| `show --stat --format=%s HEAD~1..HEAD~40` | 40 / 40 byte-identical |
| `show --format=%s HEAD~1..HEAD~40` (git `diff.renames=false`) | 28 / 40 byte-identical; 12 differ in hunk alignment (xdiff indent/compaction heuristics not implemented) |
| Live push of this evidence commit | Recorded in the commit that adds this file (pushed with `arks push`) |

## Defect found and fixed during the probe

Mixing `arks add` with stock git on one `.git` exposed stale cache-tree passthrough: arks kept the
`TREE` extension while changing entries, so stock git reported HEAD files as added and would have
committed a stale root tree. Fixed by [[sot:root/arks-index]] v0.1.2 rule 4 (spec `f8e411d`) and
code `e7b41f3`; this evidence commit itself was staged, committed and pushed with arks after the fix.

## Honesty

- Rename detection and xdiff heuristics remain **Partial**.
- `--git-dir` / `--work-tree` global options remain **Missing** (repository discovery does not read `GIT_DIR`).
- Clean-room Docker gate with a simulated proxy/CA + signing program is **not yet written**.
