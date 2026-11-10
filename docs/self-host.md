---
title: Self-host with arks
description: Run a GitHub-class self-host with only arks — git remotes plus forge API in one serve process
nav_order: 42
sections:
  - what
  - commands
  - bind
  - auth
  - vs
  - honesty
  - {c: cta, name: next}
---

# Self-host with arks

You can host a **GitHub-class** stack with the **same** `arks` CLI — git remotes **and** a forge API for PRs/issues — no separate server product, and no need to run `git` or `gh` underneath.

This is the **arks product self-host** story: localhost, intranet (LAN), and optionally the internet (WAN). Stock `git-daemon` flag parity is related but **not** the same story. Forge **client** ops are **flat** on the same binary (`arks pr`, `arks issue`, … — like `gh` without a second CLI).

## What arks does

- Speaks git-compatible clone / fetch / push on claimed transports
- Runs a **single** `arks serve` process that can listen for **HTTP, SSH, and forge API** together (product default)
- Auth for remotes aligns with stock Git: **HTTP Basic** + **HTTPS/TLS** for HTTP; **SSH keys** for SSH
- Forge API auth is for `arks hub` talking to the forge module — not a substitute for Git remote auth
- Never spawns `git` or `gh` on product paths

## Commands (product surface)

```text
arks serve [--no-http] [--no-ssh] [--no-forge] [listener flags…] <directory>
arks serve config show|set|path|validate …
```

| Command | Role |
|---------|------|
| `arks serve` | Hosting umbrella — **default enables HTTP + SSH + forge API** |
| `arks serve --no-ssh` | Omit in-process SSH |
| `arks serve --no-http` | Omit HTTP git |
| `arks serve --no-forge` | Git transports only |
| `arks serve config …` | Self-host **server** config (not `arks config`, not hub client config) |
| `arks serve-http` | **Transitional** HTTP-git spelling; not a permanent alias |
| `arks daemon` | `git://` TCP daemon (top-level; weak/no auth; **not** the GitHub-like story) |
| `arks pr` / `issue` / `auth` / … | **Flat client** forge ops (peer: `gh`) |
| `arks hub …` | **Transitional** forward to flat leaves — not day-to-day UX |
| `arks hub serve` | **Transitional** thin alias toward `arks serve` forge — not the primary host start |
| `arks upload-pack` / `arks receive-pack` | Stdio pack peers (ops SSHD today) |

There is **no** `arks self-host` command. Nested `arks serve http` / `serve ssh` / `serve forge` verbs are **not** the primary surface (flags won).

## Target journey

```text
# one host process (GitHub-class self-host)
arks serve /path/to/repos

# clients (merged git + gh surface)
arks clone http://host/repo.git          # or stock git
arks pr list --host http://host          # flat forge client
```

## Bind: localhost, LAN, WAN

| Reach | What you do | What arks owns |
|-------|-------------|----------------|
| Localhost | Default loopback-friendly demos | Git wire / forge API on enabled modules |
| LAN | Bind to a LAN address (or all interfaces) + open the host firewall | Same |
| WAN | Bind + firewall + **HTTPS/TLS in front** (HTTP) and/or SSH keys + firewall | Wire after traffic arrives |

**Ops owns:** firewall rules, DNS, TLS certificates, reverse proxies. Prefer HTTPS for HTTP off localhost. **In-process TLS** is not claimed yet. arks does **not** claim “safe on the open internet by default.”

## Auth (Git remote vs forge client)

- **HTTP git path:** HTTP Basic when configured; prefer **HTTPS/SSL/TLS** via operator-terminated TLS off-localhost
- **SSH path:** public keys / `authorized_keys`-style expectations. In-process SSH under `arks serve` is still **design / Missing** — today use ops SSHD + `upload-pack`/`receive-pack`, or wait for Claimed `serve` SSH
- **Forge API:** tokens / `arks hub auth …` against the forge module base URL
- **Fail-closed:** if a module is enabled and not ready, `arks serve` errors out — it does **not** silently pretend the default set came up. Until SSH/forge-in-serve are Claimed, use `--no-ssh` / `--no-forge` (or transitional `arks hub serve` for Partial local forge)
- **`git://` (`arks daemon`):** typically unauthenticated, no SSL — prefer `arks serve` when auth matters

## Versus other surfaces

| | Self-host (`arks serve`) | `arks daemon` | Flat forge client |
|--|--------------------------|---------------|------------------|
| Job | GitHub-class host (git + forge API) | Match `git-daemon` flags | `arks pr` / `issue` / … (peer: `gh`) |
| Required for git alias? | No (server bar) | No | No |
| Separate binary? | No | No | No |

## Honesty

Wire transports (smart HTTP, dumb HTTP, SSH stdio, `git://`) may already be claimed in Specfold with clean-room receipts. HTTP Basic (when configured) is claimed on the wire path. **In-process SSH listener**, **forge API under `arks serve`**, **in-process TLS**, **`arks serve config`**, and dedicated **self-host LAN/WAN e2e** remain **design / Partial / scaffold** — do not invent Claimed. Transitional `arks hub serve` Partial does **not** invent forge-in-serve Claimed. See Specfold [[sot:root/arks-self-host-server]].

## Next

[Getting started](../getting-started/) · [Limits](../limits/) · [arks hub](../arks-hub/) · [Git vs arks](../git-vs-arks/)
