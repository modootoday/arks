---
id: sot:root/arks-daemon-product-intent
title: arks-daemon product intent
status: active
created: 2026-11-10T19:16:00Z
updated: 2026-11-11T00:40:00Z
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
    - sot:root/arks-partial-residual-wave
    - research:root/20261111093543
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
8. **Upload-archive B5 is Claimed.** Byte-identical ustar vs stock git-daemon pin **2.39.5** evidenced by Docker `cmp -s` ([[evidence:root/20261111091600-claimed-daemon-archive]]; [[sot:root/arks-claimed-residual-gates]]; IW-B5 via [[research:root/20261111084400]] / [[research:root/20261111090827]]). Protocol/structural agreement remains a weaker Partial oracle and **MUST NOT** alone invent Claimed.
9. **Syslog B5+ is Claimed** with collector PRI receipt ([[evidence:root/20261111025206-daemon-b5-plus-syslog]]; facility daemon=3 + severity info=6 → PRI 30). Unix `openlog(LOG_DAEMON)` + `syslog(LOG_INFO, …)` marker; Windows Event Log ≠ POSIX syslog (**N/A** / Missing refuse). Gate: [[sot:root/arks-honesty-leftovers-wave]] + [[research:root/20261111114312]].
10. **IW-DAEMON-FLAGS-PARITY** ([[sot:root/arks-in-scope-leftovers-wave]]): peer-mapped flags Parity with Docker receipt ([[evidence:root/20261111034816-daemon-flags-parity]] + deepen). `service:upload-archive` enable+ustar via `git archive --remote` is Parity; path-filter / non-ustar formats Stay Partial. `related:git:// client` Stay Partial pointer (do not double-count). **Classic SysV double-fork Claimed is Refuse** — peer `git-daemon --detach` is single-fork + `setsid` ([[research:root/20261111093543]], [[research:root/20261111105924]]); arks Partial re-exec detach ≠ Claimed peer parity. **`--syslog` B5+ facility/priority Claimed** via collector (rule 9).

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
