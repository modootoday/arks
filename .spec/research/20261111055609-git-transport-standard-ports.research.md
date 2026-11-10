---
id: research:root/20261111055609
title: Git transport standard / well-known ports (IANA vs convention)
status: active
created: 2026-11-10T20:56:09Z
updated: 2026-11-10T20:56:09Z
tags: [git, ports, iana, ssh, http, https, git-daemon, self-host, arks-serve, arks-daemon, no-invent-claimed]
verify: manual
rel:
  depends_on:
    - sot:root/arks-self-host-server
    - sot:root/arks-daemon-git-command-matrix
    - research:root/20261108071916
    - research:root/20261111035226
  related:
    - docs/self-host.md
---

# Git transport standard / well-known ports

Tip surveyed: **`8a633ce`** (canon `C:\workspaces\gir`). **No invent Claimed.** This note distinguishes **IANA-registered / Git-documented normative ports** from **self-host convention / alternate** ports (especially **2222**). Recommendations for `arks serve` / `arks daemon` defaults are **design advice** only unless a follow-up SoT amend is explicitly approved.

## Question

What are the **registered/standard** ports for Git-related transports, why do operators use **2222**, and what should arks prefer as **product defaults** (normative ports vs unprivileged/LAN practice)?

## Method

| Source class | What was used |
|--------------|---------------|
| **Primary — IANA** | Service Name and Transport Protocol Port Number Registry (CSV / search): `git`→9418, `ssh`→22, `http`→80, `https`→443; also `2222` registration |
| **Primary — Git** | [git-daemon](https://git-scm.com/docs/git-daemon) (`DEFAULT_GIT_PORT` aka **9418**); pack/HTTP protocol docs via [[research:root/20261108071916]] |
| **Secondary — practice** | GitLab Docker / `gitlab-sshd` / GDK docs; Gitea rootless Docker docs (why **2222** appears) |
| **Local arks** | `crates/arks/src/cmd/server.rs` defaults; [[sot:root/arks-self-host-server]] bind rules (loopback-first; no WAN Claimed) |

## Findings — port table

| Transport | Port | IANA service | Classification | Notes |
|-----------|------|--------------|----------------|-------|
| Native `git://` | **9418** | `git` — “git pack transfer service” (tcp/udp; assignee Linus Torvalds; **2005-09**) | **Registered / normative** | git-daemon default; `/etc/services` example in man page |
| Git over SSH | **22** | `ssh` — Secure Shell (RFC 4251; also sctp) | **Registered / normative** | Git has no separate SSH port; clients omit `:port` when using 22 |
| Git smart/dumb HTTP | **80** | `http` (RFC 9110) | **Registered / normative** | Cleartext HTTP; URL may omit port |
| Git over HTTPS | **443** | `https` (RFC 9110) | **Registered / normative** | Preferred off-localhost for HTTP remotes; TLS often ops-terminated |
| Alternate SSH (common) | **2222** | **Not Git/SSH** — IANA: `EtherNet-IP-1` / EtherNet/IP I/O | **Convention / alternate only** | See §2222 below — **not** “more standard” than 22 |
| Unprivileged HTTP demo | **8080** (etc.) | Various / not Git-specific | **Convention / alternate** | Avoids privileged bind `<1024` |
| arks forge API (local) | **3920** | *(none — product-local)* | **arks convention** | Not an IANA Git port; keep documented as product-local |

### Other git-associated IANA registrations

- The only **Git-named** IANA assignment found is **`git` / 9418** (tcp + udp).
- Git **SSH** and **HTTP(S)** ride general transport registrations (**22 / 80 / 443**), not Git-specific port numbers.
- No additional IANA service names such as `git-ssh`, `git-http`, or `gitserver` were found as normative alternatives.

## Detail by transport

### 1. `git://` — 9418 (normative)

- **git-daemon:** “normally listens on port `DEFAULT_GIT_PORT` aka **9418**.”
- **IANA:** service name `git`, description “git pack transfer service,” tcp and udp.
- **arks today:** `arks daemon` default listen **`127.0.0.1:9418`** — already aligned with the registered default (loopback-first is an arks safety choice, not a port-number deviation). See [[research:root/20261111035226]].

### 2. SSH git — 22 (normative) vs 2222 (convention)

**Normative:** Git over SSH uses the **SSH** service on **port 22**. Stock clone URLs (`git@host:path` / `ssh://git@host/path`) imply **22** when no port is written.

**Why people use 2222 (not “more standard”):**

| Reason | Explanation |
|--------|-------------|
| Conflict with host `sshd` | Dedicated Git SSH listener cannot share **22** with system OpenSSH without ops coordination; publish **host:2222 → container:22** (or move host sshd) |
| Non-root / privileged ports | On Unix-like systems, binding **&lt; 1024** typically needs root or `CAP_NET_BIND_SERVICE`; unprivileged processes pick **&gt;1024** |
| Container / rootless images | e.g. Gitea **rootless** builtin SSH defaults to **2222** because the process is non-root; GitLab `gitlab-sshd` docs note privileged ports &lt;1024 are unavailable as the `git` user; GDK default listen often **2222** |
| Familiar mnemonic | “SSH-like but not 22” — widespread Docker Compose samples |

**IANA on 2222:** registered for **EtherNet/IP I/O** (`EtherNet-IP-1`), **not** SSH and **not** Git. Using 2222 for Git SSH is **operator convention**, not a standards upgrade.

**arks today:** in-process SSH path stubs / sketches use **`127.0.0.1:2222`** (Missing/Partial; fail-closed per SoT). That matches **dev/LAN alternate practice**, not the normative SSH port.

### 3. HTTP / HTTPS smart HTTP — 80 / 443 (normative)

- Smart HTTP is ordinary HTTP(S) with Git service endpoints ([[research:root/20261108071916]] / gitprotocol-http).
- **Normative ports:** **80** (http), **443** (https). Clients omit the port in URLs when these defaults apply.
- **arks today:** git HTTP listener default **`127.0.0.1:8080`** — common **unprivileged** alternate for loopback demos; **not** more standard than 80/443.
- Product honesty ([[sot:root/arks-self-host-server]]): prefer **ops-terminated TLS** for HTTPS off-localhost; in-process TLS remains Partial/Missing.

### 4. Privilege and LAN self-host practice

| Port range | Typical implication |
|------------|---------------------|
| **System ports 0–1023** (22, 80, 443) | Normative for public URLs; often need elevated bind capability or an ops front (reverse proxy / load balancer / NAT) |
| **User ports ≥1024** (8080, 2222, 3920, 9418) | Easier unprivileged bind; **9418** is still the **registered** git-daemon port (user port, not system port) |

**LAN practice that stays honest:**

1. Prefer **normative port numbers in product docs and advertised clone URLs** when the operator can arrange bind or fronting (22 / 80 / 443 / 9418).
2. Allow **explicit alternate binds** for demos and shared hosts (8080, 2222) — document them as **privilege/conflict workarounds**, never as “the standard Git port.”
3. Keep **loopback-first** defaults for safety (SoT §Bind); changing **host** from `127.0.0.1` to LAN/`0.0.0.0` is separate from choosing the **port number**.

## Implications for arks defaults (recommend only)

| Surface | Current default (tip `8a633ce`) | Recommendation | Rationale |
|---------|----------------------------------|----------------|-----------|
| `arks daemon` (`git://`) | `127.0.0.1:**9418**` | **Keep 9418** | Already IANA + git-daemon normative |
| `arks serve` HTTP git | `127.0.0.1:**8080**` | **Keep 8080 as safe unprivileged loopback default**; document **80/443** as normative LAN/WAN targets (443 via ops TLS front) | Prefer normative *story*; avoid forcing privileged bind for demos |
| `arks serve` SSH (when Claimed) | sketched `127.0.0.1:**2222**` | Prefer product language: **normative port = 22**; keep **2222 (or configurable)** as **documented alternate** when non-root or host sshd owns 22 — **do not** treat 2222 as more standard | Aligns with IANA/`ssh` and Git URL defaults; matches why GitLab/Gitea use 2222 |
| Forge API | `127.0.0.1:**3920**` | **Keep as arks-local** (or later `serve config`); not an IANA Git port | No registered forge-API port to “upgrade” to |

### Suggested SoT / docs touch (recommend; not applied here)

Light cross-link for [[sot:root/arks-self-host-server]] / `docs/self-host.md` when defaults are next edited:

> **Ports:** Prefer IANA/Git-documented ports in operator-facing clone URLs (**22** SSH, **80/443** HTTP(S), **9418** `git://`). Loopback demos MAY bind unprivileged alternates (**8080**, **2222**). **2222 is convention** (sshd conflict / non-root), not a more-standard Git port.

No Claimed change; no invent of in-process SSH readiness.

## Verdict

1. **Only Git-specific registered port:** **9418** (`git`).
2. **Git SSH / HTTP(S) normative ports:** **22 / 80 / 443** (general IANA services).
3. **2222 is not a Git standard** — it is a widespread **alternate** for privilege and host-sshd conflict; IANA assigns 2222 to EtherNet/IP.
4. **arks daemon** already prefers the normative git port; **arks serve** should keep unprivileged loopback defaults for demos but **document and prefer normative ports** for real LAN/dedicated-host self-host, with clear privilege notes.

## Sources

- IANA Service Name and Transport Protocol Port Number Registry — https://www.iana.org/assignments/service-names-port-numbers/service-names-port-numbers.xhtml (CSV entries: `git`/9418, `ssh`/22, `http`/80, `https`/443, `EtherNet-IP-1`/2222)
- Git — git-daemon: https://git-scm.com/docs/git-daemon
- GitLab Docs — Docker install / SSH port configuration; `gitlab-sshd` privileged-port note
- Gitea Docs — rootless Docker builtin SSH on **2222**
- Local: `crates/arks/src/cmd/server.rs`; [[sot:root/arks-self-host-server]]; [[research:root/20261108071916]]; [[research:root/20261111035226]]
