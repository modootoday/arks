---
id: sot:root/arks-daemon-product-intent
title: arks-daemon product intent
status: active
created: 2026-11-10T19:16:00Z
updated: 2026-11-10T23:45:00Z
tags: [arks-daemon, git-daemon, daemon, internal-crate, single-release, git-symmetry, self-host]
verify: manual
rel:
  depends_on:
    - sot:root/product-intent
    - sot:root/cli-surface
    - sot:root/crate-architecture
    - sot:root/remote-host-compatibility-surface
    - sot:root/arks-server
    - sot:root/arks-self-host-server
    - sot:root/public-surface
    - research:root/20261111035226
  evidence:
    - evidence:root/20261111035226
---

# arks-daemon product intent

Version **0.1.0**. Normative intent for **`git-daemon` symmetry** inside the **single** published CLI **`arks`**.

## Rule

### Shipping

1. **Single release = `arks` only.** There is **no** separate published `arks-daemon` binary or crates.io product release.
2. Users invoke **`arks daemon`** (stock peer: [`git-daemon`](https://git-scm.com/docs/git-daemon)).
3. Implementation MAY live in an **internal** workspace crate **`arks-daemon`** (library). The `arks` CLI dispatches into that crate. Sibling code split owns the crate cut; this SoT owns the product shape.

### Mission

4. Provide **git-daemon option/behavior symmetry** for `git://` serving — part of Git server / M11 honesty, **not** forge UX.
5. Keep daemon paths **in-engine** — MUST NOT spawn `git` or `gh`.
6. Related server porcelain (`upload-pack`, `receive-pack`, `http-backend`, `serve` / transitional `serve-http`, `update-server-info`) remains on the `arks` CLI / `arks-server` graph per [[sot:root/cli-surface]] / [[sot:root/remote-host-compatibility-surface]]; this SoT owns the **daemon peer** responsibility split. Self-host product surface is [[sot:root/arks-self-host-server]] (`arks serve` flags — not folded into daemon).
7. **Product self-host** (localhost / LAN / WAN reachability using only `arks`) is owned by [[sot:root/arks-self-host-server]]. Daemon flag symmetry is **adjacent**, not a substitute: stock `git-daemon` parity does **not** by itself define the arks self-host product story.
8. **Upload-archive B5 remains Partial (terminal for Claimed residual goal).** Protocol/structural member agreement may green; byte-identical ustar vs stock 2.39.5 is **not** Claimed (known header encoding class 501009). Opportunistic `cmp` / earlier evidence folders **MUST NOT** invent Claimed ([[sot:root/arks-claimed-residual-gates]]).
9. **Syslog B5+ is Refuse (terminal) for this residual wave.** Unix `LOG_DAEMON` Partial arming ≠ facility/priority Claimed; Windows Event Log ≠ POSIX syslog. Reopen only with collector receipt + SoT amend.

### Non-goals

1. Separate daemon product marketing or dual-bin release.
2. Forge / Checks / OAuth as part of daemon.
3. Claiming option parity without matrix/evidence ([[research:root/20261111035226]], [[evidence:root/20261111035226]]).
4. Treating full git-daemon flag Parity as “self-host Done” ([[sot:root/arks-self-host-server]]).

### Relationship

```text
Published:     arks
                 ├── … git porcelain/plumbing …
                 ├── arks daemon  ──► internal crate arks-daemon
                 └── product self-host story ──► sot:root/arks-self-host-server
Stock peer:    git-daemon   (flag symmetry only — not the self-host product narrative)
```

## Why

Stock Git ships `git-daemon` as a peer of the main CLI. arks mirrors that **command** under one release binary while allowing an internal crate boundary for maintainability. Operators who want an **arks-only** host reachable on LAN/WAN need the explicit self-host SoT — not only daemon flag inventory.

## Verification

1. Public copy never advertises a separate `arks-daemon` release.
2. `arks daemon --help` / option matrix stay honest vs pin evidence.
3. No `git`/`gh` spawn on daemon product paths.
