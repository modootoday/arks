---
id: plan:root/20261119210000
title: Cloud drop-in remaining rungs (C7–C_final + V) execution plan
status: active
created: 2026-11-19T21:00:00Z
updated: 2026-11-19T21:00:00Z
tags: [cloud, dropin, plan, c7, c8, c9, verify, flag-corpus, no-invent-pass]
verify: manual
rel:
  depends_on:
    - plan:root/20261119180000
    - sot:root/cloud-agent-dropin
    - sot:root/cli-git-help-matrix
    - adr:root/20261119180100
    - adr:root/20261119200000
---

# Cloud drop-in remaining rungs — execution plan

Refines [[plan:root/20261119180000]] after C0–C6 and C1 (CELIR removal). `Partial` in
`verify/matrices/cli-git-help-matrix.csv` means *the command exists but its option depth trails
stock git*; the full option surface of 117 commands is not one session of work, so C7 is split
into measured tiers driven by a peer-compare corpus.

## Rungs

| Rung | Scope | Exit (host advisory) | Exit (Claimed) |
|------|-------|----------------------|----------------|
| **V** | Docker gate `g042-cloud-dropin`: `arks serve` TLS with a private CA + plain CONNECT proxy + `ssh-keygen` signing + `GIT_CONFIG_COUNT`; runs the cloud probe corpus against it | Script + compose service written; shell syntax checked | Docker receipt (needs a host with a Docker daemon) |
| **C7a** | Agent tier: flag corpus `verify/fixtures/cli-flag-corpus/agent-tier-a.txt` over a deterministic fixture repo (add, commit, status, diff, log, show, branch, checkout, switch, restore, reset, rev-parse, rev-list, merge, merge-base, rebase, cherry-pick, revert, stash, tag, describe, ls-files, grep, blame, config, for-each-ref, cat-file, clean, worktree, remote) | Corpus SAME rate measured each wave; every DIFF either fixed or listed with reason | Same corpus inside the clean-room image vs pin git 2.39.5 |
| **C7b** | Remaining Partial rows by usage tier (plumbing, maintenance, mail/patch, server) | Per-command corpus lines added and measured | Clean-room corpus receipt; CSV `status_now` promoted only with evidence |
| **C8** | Daemon flag surface; hub Actions / release assets / `pr checkout` against the local forge (`arks serve` forge API) | Local-forge e2e green | Docker receipts; live GitHub stays secrets-gated (D2) |
| **C9** | Cloud rollout | Setup-script snippet verified in a session | Needs a binary source other sessions can fetch: public repo (`cargo install --git`) or a release asset — **owner decision** (D3 forbids tags in this ladder) |
| **C_final** | Register, `docs/completion.md`, roadmap tip, `cli-git-help-matrix` counts | Docs match receipts | — |

## Order

V → C7a (waves until the agent tier is SAME or reasoned) → C9 snippet → C8 → C7b → C_final.
Work stops honestly at any rung that needs Docker, secrets or an owner decision; those rows stay
Partial / Blocked with the reason recorded.

## Measurement

`verify/scripts/run-cli-flag-corpus-compare.sh <arks> <corpus> <out>` builds the fixture repo with
stock git (fixed identities and dates), then runs each corpus line twice in fresh copies — once with
`arks` (personality git), once with stock git — and records SAME / DIFF on exit code + stdout.
Stock git is the peer oracle only (rule `30`).
