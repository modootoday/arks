---
title: Git vs arks
description: Honest feature comparison — Claimed, N/A, and reopen-only residuals
nav_order: 65
schema: {faq: auto}
sections:
  - how
  - core
  - remotes
  - signing
  - out
  - {c: faq, name: reading}
  - {c: cta, name: next}
---

# Git vs arks

Side-by-side product honesty for stock **Git** vs **arks**. Status labels are Specfold-backed — **Claimed** only with receipts, **N/A** when out of claim, **reopen-only** when a residual may return via SoT amend. This page does **not** invent PASS or Claimed.

> [!NOTE]
> Authoritative ledgers live in the source tree under `.spec/` (especially the [production residual register](https://github.com/modootoday/arks/blob/main/.spec/sot/production-residual-register.sot.md) and [CLI surface](https://github.com/modootoday/arks/blob/main/.spec/sot/cli-surface.sot.md)). Clean-room Docker gates prove Verified claims — host CI alone does not.

## How to read status

| Label | Meaning |
|-------|---------|
| **Claimed** | Product surface with carrying evidence / clean-room receipt |
| **Done (ops)** | Maintainer readiness landed; not a live publish / latency PASS |
| **N/A** | Explicitly out of claim; refuse-closed or excluded |
| **Reopen-only** | Residual may return only via SoT amend + re-verify |
| **Closed (policy)** | Owner / process closed; not a development Open |

## Core CLI and same-`.git`

| Area | Git | arks | Status |
|------|-----|------|--------|
| Native CLI on same `.git` | Yes | Yes (in-engine; no product-path `git` wrapper) | **Claimed** (M9 / G14) |
| Everyday porcelain (`add`/`commit`/`status`/`log`/branch/checkout/…) | Yes | Yes for claimed inventory | **Claimed** (M9–M16) |
| Full CLI inventory + flag symmetry (pin) | Yes | Implement or N/A exclusion (M16 bar) | **Claimed** (G20 / G21) |
| `git` → `arks` alias drop-in | N/A | Documented alias; muscle-memory argv | **Claimed** (M16 / G21) |
| Clean-room journey catalog (19 scenarios) | Oracle peer | Func PASS in Docker G16 | **Claimed** (G16 / G19 venue) |
| Interactive (`rebase -i`, `add -p`/`-i`/`-e`, `stash -p`, editor commit) | Yes | TTY + hunk `-p`, `-i` menu, `-e`, `stash -p` (**FS1/FS3**); scripted env retained | **Claimed** (FS1 / FS3) |
| `proc-receive` (server) | Yes | Advertise when hook present; divert; ok/ng + rewrite; concurrent lock (**FS2/FS8**) | **Claimed** (FS2 / FS8 closest) |
| GUI (`gitk`, `git-gui`) | Separate tools | External handoff refuse (**FS7**) | **Claimed** (handoff) |
| Git LFS / `scalar` | Companion tools | In-process `arks lfs fetch|pull|push|lock|locks|unlock` + checkout smudge via batch/basic; `serve-http` `info/lfs` batch+GET/PUT + **locks**; tus/S3 handoff; other argv → `git-lfs`; `scalar` refuse | **Claimed** (locks G4; tus/S3 Partial) |
| `replace` / `--bundle-uri` | Yes | `arks replace`; local + **HTTP(S)** bundle seed (**FS5**); server v2 **`bundle-uri` advertise** (**G3**) | **Claimed** (FS5 / NET-006 / G3) |
| Object integrity / sha256 repos | Yes (sha1 + sha256) | `fsck` OidMismatch refuse + **sha1cd detect+refuse** (**OBJ-004**); `init --object-format=sha256` porcelain e2e | **Claimed** (detect+refuse) / **Impossible** (crypto immunity) |
| Cloud sync on `.git` / OneDrive placeholders | Host hazard | `arks host-env` DriveFS/OneDrive/Dropbox/Syncthing **detect + opt-in refuse**; CfAPI `RECALL_ON_DATA_ACCESS` detect; FS-014 LSM detect | **Claimed(advisory+refuse)** detect (FS-007 / WIN-003 / FS-014); full sync / hydration / LSM bypass **Impossible** |
| Forge Checks / PR / Issues UX | Not-git | `arks forge` thin wrap → host `gh`; `arks forge recipe`; credential.helper OAuth | **Claimed-closest** thin-wrap; full forge host UX **Non-goal** |

## Remotes and server

| Area | Git | arks | Status |
|------|-----|------|--------|
| Smart HTTP clone/fetch/push | Yes | Yes | **Claimed** |
| SSH clone/fetch/push (OpenSSH helper) | Yes | Yes | **Claimed** |
| Client `git://` | Yes | Yes | **Claimed** (host wire; ladder L2) |
| Dumb HTTP push (WebDAV write subset) | Yes (peer-dependent) | Yes (LOCK/MKCOL/PUT/MOVE/DELETE; `:<ref>` delete); Apache `mod_dav` twin **G41** | **Claimed** (host wire; L3 + S3 + O4 G41) |
| `push --force-with-lease` / `--push-option` | Yes | Yes on smart pack (lease + options→hooks); not on dumb | **Claimed** (O4 host wire) |
| Server smart HTTP / dumb / SSH stdio / `git://` daemon | Yes | Claimed server subset | **Claimed** (M11 / M25 G29–G32) |
| `--depth` shallow clone/fetch | Yes | Yes on pack transports | **Claimed** (G35) |
| Partial-clone filters / promisor | Yes | `blob:none`, `tree:0`, `tree:<n>`, `blob:limit=<n>[kmg]`, `object:type=…`, `combine:` of Claimed children (**FS4 + G6**); `sparse:oid` refuse | **Claimed** (R3 / S1 / O2 / FS4 / G6) |
| Forge Checks / merge-queue / Issues UX | Host APIs | Not a git CLI feature — use `gh` | **Impossible** (documented) |
| Forge OAuth browser UX | Via helpers | External `credential.helper` MAY speak OAuth; no in-arks browser UX | **Claimed** (helper honor) / **Impossible** (embedded OAuth) |

## Signing and credentials

| Area | Git | arks | Status |
|------|-----|------|--------|
| SSH commit sign / verify (`gpg.format=ssh`) | Yes | Yes (`commit -S` / `verify-commit`) | **Claimed** (G37) |
| OpenPGP / X.509 commit signing | Yes | Yes (`gpg.format=openpgp` / `x509`) | **Claimed** (R4 / S2 / G38) |
| Signed tags (`tag -s`) / `verify-tag` | Yes | Yes | **Claimed** (R4 / S2 / G38) |
| Credential helper / askpass / TTY / OS wallets | Yes | Claimed first-party subset | **Claimed** (G33) |

## Performance and concurrency

| Area | Git | arks | Status |
|------|-----|------|--------|
| Clean-room journey ≤0.8× allow-list | Oracle | Measured 19/19 | **Claimed** (M17 / G19) |
| Aggressive ≤0.5× journeys | Oracle | Historical **8/8** at `79fc9bf`; tip **Claimed(partial)** (18 + ALIAS `aggressive_na` **0.5489** after G7 PATH/`exec`; REVERT Claimed) | **Claimed(partial)** on tip (PF5 / G7) |
| Product leases / high-concurrency shape | N/A (stock) | Claimed with receipts | **Claimed** (M10 / M23 / M26) |
| Live forge latency SLO | N/A | Harness ready; no invented live PASS | **Done (ops)** (L5) ≠ live PASS |

## Out of claim / reopen-only

| Residual | Disposition | Reopen requires |
|----------|-------------|-----------------|
| R3 filter / promisor | **Claimed `blob:none` + `tree:0` + FS4 `blob:limit`/`tree:<n>`**; exotic refuse | SoT amend for further specs |
| R4 OpenPGP / X.509 / `tag -s` | **Claimed (S2 / G38; S6 re-PASS)** | Re-verify G38 after crypto wire change |
| R5 live forge scale PASS | Harness Claimed; **O1 secrets hard-block** (Environment `g8-live` secret count=0) | Secrets + receipt; never invent latency |
| Apache DAV dumb twin | **Claimed (G41 PASS)** | Re-verify after DAV/wire change |
| R6 clean-room Git pin (**2.39.5**) | Process-closed | Deliberate pin bump + G14-class re-prove |
| crates.io / docs.rs timing | Closed owner policy (L9) | Owner decision |

## Product-completion DoD (ladder)

### Historical (L0–L_final) — Done

1. **L8** — **Done**: clean-room Docker suite `g14`→`g16`→`g20`→`g21` PASS (see [matrix-l8-e2e](https://github.com/modootoday/arks/blob/main/verify/matrices/matrix-l8-e2e.md)).
2. **L_final** — **Done**: this comparison matrix ships on Pages; keep honest against Specfold residuals (never invent Claimed).
3. **L9** — Remains closed owner policy unless the owner opens it.

### Expanded symmetry DoD (S0–S_final) — Done

| Goal | Focus | Status |
|------|-------|--------|
| **S0–S_final** | See prior tip | **Done** (**L9** closed) |

### Optional residuals (O1–O_final) — historical Done

| Goal | Focus | Status |
|------|-------|--------|
| **O1** | Live G8 remasure | **Done (secrets hard-block)** — Environment secrets=0 |
| **O2** | Extra filter-specs (`tree:0`) | **Done** (host wire) |
| **O3** | Interactive scripted subset | **Done** (superseded for TTY by **FS1**) |
| **O4** | force-with-lease / push-options / Apache DAV | **Done** (lease+options + G41 PASS) |
| **O_final** | Docs / residual currency; **L9** closed | **Done** |

### Full Symmetry ladder (FS0–FS_final) — Done

DoD = **complete git feature symmetry** (not “O-ladder Done with casual N/A forever”). **L9** and live G8 latency remain non-FS.

| Goal | Focus | Status |
|------|-------|--------|
| **FS0** | Inventory all asymmetric rows | **Done** |
| **FS1** | TTY `add -p` + `rebase -i` editor fallback | **Done** |
| **FS2** | `proc-receive` advertise + divert | **Done** |
| **FS3** | Hunk `-p` / `add -i`/`-e` / `stash -p` | **Done** |
| **FS4** | Extra filter-specs | **Done** |
| **FS5** | replace / HTTP+local bundle-uri / info lock | **Done** (NET-006 Claimed) |
| **FS6** | filter clean/smudge + in-process LFS / scalar | **Done** |
| **FS7** | GUI handoff / OAuth helper | **Done** |
| **FS8** | Hostile/impossible + OBJ-004 fsck | **Done** |
| **FS_final** | No casual N/A for real git features | **Done** — remaining = documented Impossible / advisory / owner-policy only |

### Completeness honesty (tip ≥ `54c9dc2` / roadmap tip ≥ `60f33c2`)

| Question | Verdict |
|----------|---------|
| Git feature symmetry DoD (**FS_final**) | **YES** — Claimed rows have receipts; residuals are Impossible/advisory/owner-policy, not casual N/A |
| Product “everything including live forge SLOs / crates.io publish / crypto immunity” | **NO** — those stay documented non-goals (see below) |
| Operator one-word | **PARTIAL** — see [Completeness](../completion/) |
| Clean-room re-stamp of all Docker gates on this tip | See evidence pack under `.spec/evidence/*-full-cleanroom-matrix/` (maintainer); host CI ≠ Verified |

**Documented non-goals / blocked (not FS gaps):** absolute SHA-1 collision immunity; full forge Checks/merge-queue/Issues product UX (thin `arks forge`→`gh` is Claimed-closest only); embedded forge OAuth browser UX; full multi-device cloud-sync safety on `.git` (FS-007/WIN-003/FS-014 detect+opt-in refuse only); live G8 latency PASS without secrets; **L9** crates.io/docs.rs timing (owner policy — reopen pack only).

See: [program roadmap tip currency](https://github.com/modootoday/arks/blob/main/.spec/sot/program-roadmap.sot.md) · [partial unblock research](https://github.com/modootoday/arks/blob/main/.spec/research/20261111001816-partial-completion-unblock-roadmap.research.md) · [Full Symmetry FS ladder](https://github.com/modootoday/arks/blob/main/.spec/research/20261110212414-full-symmetry-ladder-fs0-fs-final.research.md) · [O-ladder](https://github.com/modootoday/arks/blob/main/.spec/research/20261110205403-optional-residuals-o-ladder-o1-o-final.research.md) · [S-ladder](https://github.com/modootoday/arks/blob/main/.spec/research/20261110105436-git-symmetry-expanded-dod-goal-ladder-s0-s-final.research.md). Limits: [Limits](../limits/).

## Next

[Completeness](../completion/) [Limits](../limits/) [Performance](../performance/) [Alias `git` → `arks`](../alias-git-arks/) [Contribute](../contribute/)
