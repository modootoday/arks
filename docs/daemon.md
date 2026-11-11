---
title: arks daemon
description: git-daemon symmetry peer — not the GitHub-class self-host story
nav_order: 44
sections:
  - what
  - vs
  - honesty
  - {c: cta, name: next}
---

# arks daemon

`arks daemon` is the **`git-daemon` peer** on the **same** published `arks` binary. It serves the native **`git://`** transport (default listen **9418** on loopback-friendly binds).

It is **not** the product GitHub-class host. For HTTP/SSH/forge API hosting, use [`arks serve`](../self-host/).

## What it is

- Single release: still just **`arks`** (internal library may exist; no separate `arks-daemon` product binary)
- Flag / behavior symmetry vs stock `git-daemon` is a clean-room **DoD pillar** (pillar 2)
- Must not spawn `git` or `gh` on product paths
- Typically weak or no auth / no SSL — prefer `arks serve` when auth matters

```bash
arks daemon --help
arks daemon --listen=127.0.0.1 --port=9418 /path/to/repos
```

## Versus `arks serve`

| | `arks daemon` | `arks serve` |
|--|---------------|--------------|
| Peer | `git-daemon` | GitHub-class self-host |
| Transports | `git://` | HTTP (+ SSH + forge API modules) |
| Auth story | Weak / usually none | HTTP Basic + ops/in-process TLS (**B9**); SSH keys (**B1/B2** Claimed where evidenced) |
| Required for git alias DoD? | No | No |

## Honesty

Daemon flag inventory is largely present as **Partial** deepen: `--inetd` / `--pid-file` / `--detach` (unix `setsid` / Windows DETACHED_PROCESS re-exec + parent pid-file write; not classic double-fork Claimed) / upload-archive **ACK + sideband-64k** with `--prefix=` / includes / `--exclude=` Partial (**path-filter applied in stream body** + format honesty tar/ustar; expanded protocol-body unit tests including zip ERR refuse + empty-tree ustar) / `--access-hook` stub body / unix **`--syslog`** (openlog LOG_DAEMON Partial) / unix **`--user` / `--group`** (setgid then setuid Partial; refuse fail-closed on error). Windows: `--syslog` / `--user` / `--group` honest **Missing** refuse (Event Log ≠ syslog and not wired; no setuid/setgid — do not invent Partial).

Residual honesty ([[sot:root/arks-claimed-residual-gates]]):

| Gate | Class | Notes |
|------|-------|-------|
| **B5** archive format parity | **Partial** (reopen design-gated) | Protocol/structural OK; **byte-identical ≠ Claimed** until IW-B5 Spec+research+`cmp -s` ([[research:root/20261111084400]], [[research:root/20261111090827]]) |
| **B5+** syslog facility/priority | **Refuse (terminal)** | Partial `openlog`/`LOG_DAEMON` ≠ facility Claimed; Windows Event Log ≠ POSIX syslog |

**Partial ≠ Parity Claimed.** Do not treat daemon flag completeness as “self-host Done.”

Owning SoTs: [daemon product intent](https://github.com/modootoday/arks/blob/main/.spec/sot/arks-daemon-product-intent.sot.md), [daemon command matrix](https://github.com/modootoday/arks/blob/main/.spec/sot/arks-daemon-git-command-matrix.sot.md), [claimed residual gates](https://github.com/modootoday/arks/blob/main/.spec/sot/arks-claimed-residual-gates.sot.md).

## Next

[Self-host](../self-host/) [Ports](../ports/) [Completeness](../completion/) [Limits](../limits/)
