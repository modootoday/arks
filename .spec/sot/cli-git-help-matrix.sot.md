---
id: sot:root/cli-git-help-matrix
title: CLI git-help matrix (canonical)
status: active
created: 2026-11-10T19:05:00Z
updated: 2026-11-10T19:20:00Z
tags: [cli, matrix, git-help, parity, m16, pin-2.39.5, sot]
verify: manual
rel:
  depends_on:
    - sot:root/cli-surface
    - sot:root/cli-flag-parity-complete
    - sot:root/clean-room-git-pin-advance
    - sot:root/product-intent
    - research:root/20261111034805
    - research:root/20261111035226
    - research:root/20261111035700
  evidence:
    - evidence:root/20261111034600
    - evidence:root/20261111035226
---

# CLI git-help matrix (canonical)

Version **0.1.0**. **Canon (source of truth)** for pin Git **2.39.5** command inventory vs `arks` product CLI. Machine table: `verify/matrices/cli-git-help-matrix.csv`. Survey method/evidence: [[research:root/20261111034805]], [[evidence:root/20261111034600]]. Daemon flag depth: [[research:root/20261111035226]]. Over-design trim: [[research:root/20261111035700]].

**Pin oracle:** Docker `arks-verify-m1:0.1.0` stock `git version 2.39.5` — `git --list-cmds=main,others,alias` (n=**166**). Host Git is skew-only, not the judge (observed host **`git version 2.56.0.windows.4`** — inventory signal only; pin DoD stays 2.39.5).

## Rule

1. Every pin `list-cmds` name MUST appear in the machine CSV with:
   - `category` — porcelain | plumbing | server | ancillary | synonym | merge-backend | remote-helper | gui-or-internal
   - `status_now` — live HELP/dispatch honesty at last survey tip (Parity-candidate | Partial | Missing | Refuse/N/A)
   - `arks_target` — **must-implement** | **refuse-with-reason** | **synonym→\<cmd\>**
   - `priority` — implementation backlog order (1=synonym/wire, 2=high-traffic Missing, …)
   - `notes` — short honesty
2. **M16 honesty:** public “full CLI” claims MUST match this matrix + live binary. A Missing row is **not** an N/A exclusion.
3. **Refuse** only for true GUI / git-internal `--*` helpers / explicit product exclusions already on the pin refuse set (e.g. `filter-branch`, `difftool`, `mergetool`, `instaweb`).
4. **Server KEEP:** `daemon`, `upload-pack`, `receive-pack`, `http-backend`, `update-server-info`, and plain-git `serve-http` remain in-scope (git paths only).
5. **Out-of-matrix (trimmed):** `forge`, `host-env`, in-process `lfs`, CELIR product Claimed path, `arks-hostfs` product surface — not pin git builtins; see [[research:root/20261111035700]].
6. Promoting `status_now` requires evidence on the carrying tip — **no invent Claimed** from HELP alone.

## Counts (pin n=166)

Survey baseline (`daf7f8a`): Missing **70** / Partial **60** / Parity-candidate **21** / Refuse **15**.

**Currency (SoT tip `c297e34` + host-skew refresh — pin inventory Missing=0; flag-depth still Partial):**

Living CSV includes pin rows + host **2.54/2.56** skew inventory (`format-rev`, `url-parse`, …) as Partial — **not** a pin advance. Normative DoD judge remains clean-room **2.39.5**.

| status_now | Count (living CSV) |
|------------|------:|
| Parity-candidate | 21 |
| Partial | **117** |
| Missing | **0** |
| Refuse/N/A | **47** |
| Synonym-wired | 4 |
| arks-only | 6 |

| arks_target | Count |
|-------------|------:|
| must-implement | (Partial + Parity-candidate + remaining Missing must-implement) |
| synonym→ | 4 (`stage`→`add`, `init-db`→`init`, `fsck-objects`→`fsck`, `whatchanged`→`log`) — **wired** |
| refuse-with-reason | **34** (GUI/internal + merge backends + remote helpers + `credential-cache` + `imap-send`/`quiltimport`/`pickaxe`) |

**Landed Missing→Partial (cumulative):** prior batch + `bugreport`, `diagnose`, `pack-redundant`, `shell`, `upload-archive` (+ 4 synonyms).

**Refuse honesty:** merge-strategy backends, `remote-*` helpers, `credential-cache`, `imap-send`, `quiltimport`, `pickaxe` (option not cmd).

## Implementation backlog (remainder Missing → Partial → flag gaps)

1. ~~Synonyms (4)~~ **done**
2. ~~High-traffic / ancillary / remainder Missing~~ **done (Partial subsets)** — pin inventory Missing **0** at tip carrying this slice (`commit-graph`…`subtree` included; flag-depth still Partial)
3. **Daemon option gaps** per [[research:root/20261111035226]] (was Missing ≈25; living CSV now Missing=0 via Partial+Refuse — deepen Partial→Parity); intent SoT [[sot:root/arks-daemon-product-intent]]
4. **Partial flag depth** — [[sot:root/cli-flag-parity-complete]]
5. **DoD pillars** — umbrella [[sot:root/verify-cleanroom-symmetry-matrix]]; compose `dod-cli-matrix` / aliases `run-symmetry-*.sh` (strict via `ARKS_DOD_STRICT=1`)

## Currency

- Survey CSV: `.spec/evidence/20261111034600-git-arks-cli-help-crosscheck/matrix-pin-git-vs-arks.csv`
- Living machine matrix: `verify/matrices/cli-git-help-matrix.csv` (update `status_now` when dispatch lands)
- Clean-room DoD umbrella: [[sot:root/verify-cleanroom-symmetry-matrix]] (pillar 1)
- Align [[sot:root/cli-surface]] / public docs: pin **command** inventory Missing=0 ≠ full man-page flag Claimed (Partial remains). Host Git skew ≠ pin advance.

## Verification

1. Pin `git --list-cmds=main,others,alias` count matches matrix rows (166).
2. Every Missing row either gains dispatch or moves to refuse-with-reason / synonym with rationale.
3. `arks --help` lists implemented inventory; trimmed non-git commands are absent or refuse-with-reason.
4. No G19 thrash; no invent PASS.
