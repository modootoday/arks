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
| Auth story | Weak / usually none | HTTP Basic + ops TLS; SSH keys (Partial bind+paths; handshake Claimed later) |
| Required for git alias DoD? | No | No |

## Honesty

Daemon flag inventory is largely present as **Partial** deepen: `--inetd` / `--pid-file` / `--detach` (unix `setsid` / Windows DETACHED_PROCESS re-exec + parent pid-file write; not classic double-fork Claimed) / upload-archive **ACK + sideband-64k band-1 data + band-2 progress** are Partial (path filters refuse closed). Still **Missing**: Claimed format-flag/path-filter archive parity, `--access-hook` body, user/group/syslog. **Partial ≠ Parity Claimed.** Do not treat daemon flag completeness as “self-host Done.”

Owning SoTs: [daemon product intent](https://github.com/modootoday/arks/blob/main/.spec/sot/arks-daemon-product-intent.sot.md), [daemon command matrix](https://github.com/modootoday/arks/blob/main/.spec/sot/arks-daemon-git-command-matrix.sot.md).

## Next

[Self-host](../self-host/) [Ports](../ports/) [Completeness](../completion/) [Limits](../limits/)
