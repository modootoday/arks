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

Daemon flag inventory: peer-mapped flags **Parity** (IW-DAEMON-FLAGS-PARITY); `service:upload-archive` enable+ustar **Parity** via `git archive --remote` (path-filter / non-ustar Stay Partial) + **B5 Claimed** byte-identical ustar; `related:git:// client` Stay Partial pointer. Refuse: `--inetd` / `--pid-file` / **classic SysV double-fork** (peer `--detach` is single-fork+`setsid`) / `--access-hook`. Unix **`--syslog`** (**B5+ Claimed** facility/priority with collector PRI). Windows: `--syslog` / `--user` / `--group` honest **Missing** refuse (Event Log ≠ syslog).

Residual honesty ([[sot:root/arks-claimed-residual-gates]] / [[sot:root/arks-remaining-incompletes-wave]]):

| Gate | Class | Notes |
|------|-------|-------|
| **B5** archive format parity | **Claimed** | Docker `cmp -s` vs stock git-daemon **2.39.5** ([[evidence:root/20261111091600-claimed-daemon-archive]]) |
| **B5+** syslog facility/priority | **Claimed** | Docker collector PRI=30 (`LOG_DAEMON`+`LOG_INFO`); Windows Event Log ≠ POSIX syslog (N/A) |
| Classic double-fork | **Refuse (terminal)** | Peer detach is single-fork+`setsid` |

**Partial ≠ Parity Claimed.** Do not treat daemon flag completeness as “self-host Done.”

Owning SoTs: [daemon product intent](https://github.com/modootoday/arks/blob/main/.spec/sot/arks-daemon-product-intent.sot.md), [daemon command matrix](https://github.com/modootoday/arks/blob/main/.spec/sot/arks-daemon-git-command-matrix.sot.md), [claimed residual gates](https://github.com/modootoday/arks/blob/main/.spec/sot/arks-claimed-residual-gates.sot.md).

## Next

[Self-host](../self-host/) [Ports](../ports/) [Completeness](../completion/) [Limits](../limits/)
