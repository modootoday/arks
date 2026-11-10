---
id: sot:root/arks-self-host-server
title: arks product self-host server
status: active
created: 2026-11-10T19:40:00Z
updated: 2026-11-11T00:20:00Z
tags: [self-host, server, serve, serve-flags, serve-config, daemon, lan, wan, bind, ports, auth, http-basic, tls, ssh, forge-api, product-hosting, single-release, no-invent-claimed, ssh-durable-partial, claimed-residual]
verify: manual
rel:
  depends_on:
    - sot:root/product-intent
    - sot:root/arks-server
    - sot:root/arks-daemon-product-intent
    - sot:root/arks-hub-product-intent
    - sot:root/remote-host-compatibility-surface
    - sot:root/cli-surface
    - sot:root/public-surface
    - sot:root/verify-cleanroom-symmetry-matrix
    - sot:root/arks-claimed-residual-gates
    - sot:root/program-roadmap
    - sot:root/production-residual-register
    - adr:root/20261109224236
    - research:root/20261111055609
    - research:root/20261111074523
  evidence:
    - evidence:root/20261108231300
    - evidence:root/20261109221500
    - evidence:root/20261109223000
    - evidence:root/20261109224717
    - evidence:root/20261110233659-claimed-rebind
    - evidence:root/20261110235116-claimed-ssh
  governs:
    - docs/self-host.md
---

# arks product self-host server

Version **0.2.3**. Normative **product hosting story** (document revision — **not** a product release; workspace crate remains `0.0.0`): run a **GitHub-class self-host** using **only the published `arks` binary** — **git remotes (HTTP/SSH)** plus **forge API (PR/issue/…)** in **one** `arks serve` process — reachable from **localhost**, **intranet (LAN)**, and/or **internet (WAN)** when the operator chooses bind + ops fronting.

**Honesty:** This SoT makes the story **EXPLICIT**. **Design intent ≠ Claimed.** Do not invent Claimed from this SoT alone. Wire/unit ownership for Git transports remains [[sot:root/arks-server]]. Daemon **flag** symmetry remains [[sot:root/arks-daemon-product-intent]] / [[sot:root/arks-daemon-git-command-matrix]]. **`arks hub`** is the **forge client** ([[sot:root/arks-hub-product-intent]]) — not the primary way to start the host.

**Advice amend (v0.2.0):** Earlier guidance that optimized noun symmetry (“keep forge under `hub serve` / never put forge under `arks serve`”) is **superseded**. Operators want **product unity**: one host command that feels like a self-hosted GitHub, with `arks hub` as the `gh`-like client against that base URL.

## Rule

### Mission

1. **arks-only GitHub-class self-host.** Operators MUST be able to host repositories **and** a forge API for stock Git (and arks-as-client) clone/fetch/push **and** `arks hub …` PR/issue workflows using **only** `arks` product paths — no runtime spawn of `git` or `gh`.
2. **Single hosting process under `arks serve`.** Prefer **one process** that can run Git HTTP, Git SSH, and forge API modules together (product default), not separate “git host” vs “hub listen” as the primary operator story.
3. **Product commands (v1 surface):**

   ```text
   arks serve [--no-http] [--no-ssh] [--no-forge] [listener flags…] <directory>
   arks serve config show|set|path|validate …
   ```

   | Surface | Role | Claimed honesty |
   |---------|------|-----------------|
   | `arks serve` | Hosting umbrella; **default enables HTTP + SSH + forge API** | HTTP git wire Claimed where G15/G29/G36 evidence; SSH listener **Missing/Partial**; forge-in-serve **Missing/Partial** (see §Fail-closed); hosting e2e design |
   | `arks serve --no-forge` | Git transports only (HTTP/SSH per other flags) | Same git honesty; forge module off |
   | `arks serve --no-http` / `--no-ssh` | Opt out of a Git transport | Fail-closed if remaining enabled module Missing |
   | `arks serve config …` | Self-host **server** configuration (`show` / `set` / `path` / `validate`) | **Design / Partial** — help stub OK; not Claimed. **≠** `arks config`. **≠** hub *client* settings |
   | `arks serve-http` | Transitional HTTP-git-only spelling | Migration convenience — treat as `arks serve --no-ssh --no-forge` (or documented equivalent); **not** a permanent MUST alias |
   | `arks daemon` | `git://` TCP daemon (**top-level**) | Flag symmetry peer; **not** the GitHub-like product story |
   | `arks upload-pack` / `arks receive-pack` (/ `arks shell`) | Stdio pack peers | Current SSH *wire* when **ops SSHD** invokes them — does **not** invent in-process `serve` SSH Claimed |
   | `arks http-backend` / `arks update-server-info` | When deploy path needs them | Per [[sot:root/arks-server]] |
   | `arks pr` / `issue` / `auth` / … | **Flat forge client** (gh peer) against forge base URL | See [[sot:root/arks-hub-product-intent]] v0.4 flat-first |
   | `arks hub …` | **Transitional** forward to flat leaves / `serve` | Demoted CLI noun; not day-to-day UX |
   | `arks hub serve` / `hub listen` | **Deprecated thin alias** → `arks serve` forge module | Transition only; MUST NOT remain the primary host story |

4. **Single release.** Hosting is part of the **`arks`** binary. No separate `arks-self-host` / `arks-daemon` published product. **No** `arks self-host` CLI noun.
5. **Reachability is intentional.** Localhost-only defaults are the safe default. Binding for **LAN** and **WAN** exposure is an **in-scope product goal**.
6. **Self-host ≠ 4th DoD pillar.** Related hosting under [[sot:root/verify-cleanroom-symmetry-matrix]] stays adjacent to the three peer-symmetry pillars; do not invent a fourth Claimed pillar from this SoT. Pillar 3 (`arks hub` client matrix) talks **to** this host’s forge module when self-hosting locally.

### CLI verdict — flags win; serve is the hosting umbrella

7. **Chosen shape (normative):** module/transport selection is **flags on `arks serve`**, not primary subcommands `arks serve http` / `arks serve ssh` / `arks serve forge`.
8. **Default = GitHub-class set.** With no disable flags, **HTTP + SSH + forge API** are all enabled in one process (product intent).
9. **Disable flags (preferred clap idiom):** **`--no-http`**, **`--no-ssh`**, **`--no-forge`** opt out of the default set. Do **not** require redundant enable flags for the common case (enable flags MAY exist later as aliases, but MUST NOT be the documented primary).
10. **Illegal:** disabling every module (nothing to serve) → usage error.
11. **`arks serve config`** remains a **nested subcommand** (first token `config`), not a flag. Detect before flag parse: `arks serve config …`.
12. **Superseded prose:** (a) earlier drafts that preferred `arks serve http` / `arks serve ssh` as primary verbs; (b) advice that **forbid** forge under `arks serve` and required `arks hub serve`/`listen` as the primary forge host — both **superseded** by this umbrella model.
13. **Transitional `serve-http`:** MAY remain briefly as HTTP-git-only so matrices do not break overnight. Prefer updating call sites. **MUST NOT** lock permanent MUST alias.
14. **Transitional hub host verbs:** `arks hub serve` / any `hub listen` spelling MAY remain as **thin aliases** that start (or document) the forge module under `arks serve`, then deprecate. Primary docs MUST teach `arks serve`.

### Fail-closed when a requested module is not ready

15. **Product intent:** default set means the process **attempts** each enabled module.
16. **Honesty / Partial:** in-process **SSH listener** Missing/Partial until implemented + evidenced. **Forge API in-process under `arks serve`** Missing/Partial until wired + evidenced (today’s Partial scaffold may still live under transitional `arks hub serve` — that does **not** invent forge-in-serve Claimed). In-process **TLS** remains Partial/Missing (ops reverse-proxy is the documented HTTPS pattern).
17. **Fail-closed (normative):** if a module is **enabled** (default or remaining after disable flags) and that listener/API is **not Claimed/ready**, `arks serve` MUST **not** silently degrade while advertising default-set **Claimed** success.
    - **(a) Silent degrade forbidden:** dropping an enabled module and continuing as if the default set came up Claimed-ready is illegal.
    - **(b) Bind failure fail-closed:** failure to bind an enabled listener MUST error explicitly (no half-up Claimed success).
    - **(c) Partial durable listen allowed (v0.2.2 tradeoff):** an enabled module MAY keep a **durable** accept loop when it **explicitly labels Partial** on stderr/operator surfaces and **never** advertises Claimed readiness / OpenSSH-exact / hosting e2e. Short probe-then-exit Partial remains valid for smokes. HTTP rebind now has a **Claimed bounded dual-bind drain scenario**: held old connection plus fresh old/new peers completed in Docker ([[evidence:root/20261110232045-claimed-rebind]]). SSH and forge rebind remain Partial unless separately evidenced.
    - **(d) Tradeoff recorded:** durable Partial SSH listen unlocks git-over-SSH smoke closer to e2e and serve-config rebind parity with HTTP/forge Partial — at the cost of operators seeing a long-lived Partial process (not a 2s probe). **B1 Claimed** for OpenSSH ChaCha wire and **B2 Claimed** for stock `git clone ssh://` ([[evidence:root/20261110235116-claimed-ssh]]). SSH **rebind** and process-label Claimed readiness remain Partial/Missing.
18. **Practical today:** operators run e.g. `arks serve --no-ssh --no-forge …` for Claimed HTTP git wire; or keep using transitional `arks hub serve` for Partial local forge API until forge-in-serve lands. Ops SSHD + stdio pack remains a valid **separate** deploy path and does not satisfy in-process `serve` SSH Claimed. Durable Partial SSH (when enabled) labels Partial and stays fail-closed on bind failure / silent degrade.
19. When modules are Claimed: failure to bind any enabled listener MUST fail closed (no half-up “success”).

### What arks owns vs ops

| Concern | Owner | Normative note |
|---------|-------|----------------|
| Git wire (upload-pack / receive-pack) on claimed transports | **arks** | [[sot:root/arks-server]], M11 G15 / M25 G29–G32 |
| Forge API (PR/issue/… host) under `arks serve` | **arks (future in umbrella)** | **Missing / Partial** until Claimed; client is `arks hub` |
| Listen / bind (HTTP and/or SSH and/or forge HTTP) | **arks** | Default loopback-first; LAN/`0.0.0.0` intentional when chosen |
| In-process HTTP Basic (git remote) | **arks** | G36 Claimed when configured |
| In-process SSH listener (`arks serve` without `--no-ssh`) | **arks (B1+B2 Claimed wire/clone; listen label Partial)** | Durable accept loop still Partial-labeled (§17(c)); **B1** ChaCha + **B2** `git clone ssh://` Claimed ([[evidence:root/20261110235116-claimed-ssh]]); ops SSHD still valid |
| Pack protocol on SSH stdio | **arks** | Ops supplies SSHD today |
| `arks serve config` store | **arks (design)** | Partial until Claimed |
| Firewall / DNS | **ops** | Not Claimed |
| TLS certificates / HTTPS termination | **arks + ops** | **B9 product in-process TLS Claimed** ([[evidence:root/20261110233234-claimed-tls]]); ops reverse-proxy remains a valid alternate and does not by itself satisfy B9 |
| WAN “secure by default” | **Forbidden** without TLS/keys + ops honesty | |

### Bind / listen (normative)

20. **Localhost.** Default bind remains loopback-friendly when the operator does not opt into exposure.
21. **LAN / WAN.** Operator choice: bind + firewall + (HTTP) HTTPS/TLS front + auth; (SSH) keys + firewall. arks owns Git wire / forge API after packets arrive; no WAN threat-model Claimed completeness.
22. **Daemon listen shape** stays under [[sot:root/arks-daemon-git-command-matrix]] — do not invent Parity from this hosting SoT. Daemon is **optional/legacy symmetry**, not the GitHub-like narrative.

### Default ports (normative policy)

Cite [[research:root/20261111055609]] (IANA / Git-documented vs convention). **Port policy ≠ Claimed** for in-process SSH or WAN-safe hosting.

| Transport / surface | Normative product target | Explicit alternate / demo | Classification |
|---------------------|--------------------------|---------------------------|----------------|
| Git over SSH (`arks serve` SSH module) | **22** | **2222** | 22 = IANA `ssh` / stock Git URL default; **2222** = unprivileged / host-`sshd` conflict workaround — **not** a more-standard Git port (IANA assigns 2222 to EtherNet/IP) |
| Git smart/dumb HTTP | **80** | **8080** | 80 = IANA `http`; **8080** = unprivileged bind alternate for loopback demos |
| Git over HTTPS | **443** | ops TLS front on 80/8080 (or future in-process TLS) | 443 = IANA `https`; prefer ops-terminated TLS off-localhost |
| Native `git://` (`arks daemon`) | **9418** | — | IANA `git` + git-daemon `DEFAULT_GIT_PORT`; already the product listen default |
| Forge API (under `arks serve` / transitional hub) | **3920** | configurable (`--forge-listen`) | **arks-local convention** — not an IANA Git port |

23. **Normative story.** Operator-facing docs and advertised clone URLs SHOULD prefer **22 / 80 / 443 / 9418** when the operator can arrange privileged bind, capability, or an ops front (reverse proxy / NAT / publish map).
24. **Demo / unprivileged story.** Loopback and non-root demos MAY bind **8080** (HTTP) and **2222** (SSH). Help text and docs MUST label these as **alternate / demo** (privilege or host-sshd conflict), never as “the standard Git port.”
25. **Runtime clap defaults (honesty).** Changing process bind defaults from unprivileged demos (**8080** / **2222**) to privileged normative ports (**80** / **22**) is a **coordinated** code + test + docs change. Until that lands, clap MAY keep safe loopback demos — but **MUST NOT** present 8080/2222 as more normative than 80/22 in `--help` or public docs. Daemon **9418** already matches normative.
26. **Forge 3920** stays product-local; do not invent an IANA “upgrade.”

### Auth model — Git remotes vs forge client

Self-host **Git-remote authentication** MUST align with how stock Git clients already authenticate remotes. Forge **API** auth (tokens / hub login) is for `arks hub` talking to the forge module — **not** a substitute for Git remote auth on clone/fetch/push.

| Mechanism | Status | Notes |
|-----------|--------|-------|
| Unauthenticated loopback HTTP (git) | Allowed when Basic unset | G15 demo path |
| HTTP Basic on smart HTTP (git) | **In-scope v1** (G36 when configured) | `--basic-auth` / `ARKS_HTTP_BASIC` / file |
| HTTPS / SSL/TLS | **In-process PEM (B9 Claimed) or ops front** | Product `--tls-cert/--tls-key` evidenced ([[evidence:root/20261110233234-claimed-tls]]); ops reverse-proxy still valid |
| SSH keys / `authorized_keys`-style | **Design for in-process `serve` SSH**; **wire via ops SSHD today** | Align with Git SSH remotes |
| Hub token / forge auth for **forge API** | **In-scope under forge module** | Client: `arks hub auth …`; ≠ primary git-remote auth |
| Hub token as primary **git-remote** auth | **Refuse** | Keep Git-remote vs forge-API auth split honest |
| `git://` (`arks daemon`) | Weak / typically no auth, no SSL | Prefer `arks serve` when auth matters |

27. Product paths MUST NOT spawn `git` or `gh` for hosting.
28. Secrets stay out of the tree ([[sot:root/public-surface]]).

### `arks serve config` (server configuration)

29. **Purpose.** Configure the **self-host server** (not a git working-tree):

    ```text
    arks serve config show
    arks serve config set <key> <value>
    arks serve config path
    arks serve config validate
    ```

30. **In-scope keys (design).** Bind/listen **including port policy** (normative vs demo), which modules enabled (http/ssh/forge), TLS/SSL front notes (or future in-process TLS paths), HTTP Basic credential/file refs, SSH host-key / authorized-principals paths, forge data dir / API listen, allow-paths / repo roots. Schema MAY deepen with evidence — do not invent Claimed completeness.
31. **Refuse conflation** with `arks config` (git-config symmetry) and with `arks hub config` (client forge settings).
32. **Honesty:** Partial/Missing until implemented + evidenced. Thin `--help` / “not claimed” stubs allowed.

### Relationship map

```text
arks product self-host (THIS SoT) — GitHub-class host
  ├── arks serve                         # default: HTTP + SSH + forge API
  │     ├── --no-http / --no-ssh / --no-forge
  │     └── config show|set|path|validate
  ├── serve-http                         # transitional ≡ serve --no-ssh [--no-forge]
  ├── arks daemon                        # git:// top-level; symmetry peer ≠ product story
  ├── upload-pack / receive-pack / shell # ops-SSHD stdio wire today
  ├── clients:
  │     ├── arks / git                   # remotes → git HTTP/SSH
  │     └── arks pr|issue|auth|…         # flat forge client → forge API base URL
  ├── transitional: arks hub … / hub serve|listen → flat client or serve forge module
  └── arks agent (future; optional)
```

33. Stock `git` / `git-daemon` / `gh` are peers/oracles, not the arks self-host product story.

### Non-goals

1. Forge **web UI** / Checks / merge queue as the hosting Claimed bar (API host is in-scope; UI is not required for v1 Claimed).
2. “Safe on the open internet by default” without TLS/keys + firewall honesty.
3. Inventing Claimed in-process SSHD, in-process TLS, or forge-in-serve from this SoT alone.
4. Silent degrade when an enabled module is Missing/Partial (must fail closed per §17 — Partial durable listen must label Partial; never invent Claimed success).
5. Primary nested verbs `arks serve http` / `arks serve ssh` / `arks serve forge` (flags win).
6. Permanent MUST `serve-http` alias; permanent primary `arks hub serve` as the host story.
7. Folding self-host into `arks daemon`; inventing `arks self-host`.
8. Conflating `arks serve config` with `arks config` or hub **client** settings.
9. Treating self-host as a 4th clean-room DoD pillar.
10. Inventing Claimed / PASS without clean-room evidence.
11. Merging unrelated pillars wrongly: daemon flag-parity matrix stays separate; hub **client** gh-matrix stays pillar 3 — only the **host process** unifies under `arks serve`.
12. Treating **2222** or **8080** as more-standard Git ports than **22** / **80** / **443**.
13. Silently flipping clap bind defaults to privileged ports without updating tests and help together.

### Verification (design → Claimed)

34. **Design intent (now):** this SoT + `docs/self-host.md` + honest CLI help for `arks serve` / `arks serve config` / fail-closed Partial modules; hub SoTs say **client-only** for primary UX; **port policy** (normative vs demo) documented.
35. **Already carrying (wire, not hosting e2e):** G15 / G29–G32 / G36 — cite residual register; do not re-stamp. Ops-SSHD stdio ≠ in-process serve SSH Claimed. Transitional `arks hub serve` Partial ≠ forge-in-serve Claimed.
36. **Future hosting e2e:** compose `dod-self-host` → `verify/scripts/run-self-host-e2e.sh` proving arks-only `arks serve` (with explicit disable flags until SSH/forge Claimed), LAN-shaped bind, Basic, stock Git client + optional `arks hub` against forge base — scaffold/SKIP until Claimed; **no invent hosting Claimed**.
37. **Claimed residual deepen (adjacent):** Docker slots under [[sot:root/arks-claimed-residual-gates]] — **B1+B2** ([[evidence:root/20261110235116-claimed-ssh]]), **B7 HTTP** ([[evidence:root/20261110233659-claimed-rebind]]), **B9** ([[evidence:root/20261110233652-claimed-tls]]). Partial durable SSH (§17(c)) ≠ SSH rebind Claimed; **B5+** syslog **Refuse terminal**.

## Why

Operators want **one** hosting command (`arks serve`) that feels like a self-hosted GitHub: Git remotes **and** PR/issue forge API together — not a fragmented “git serve here, hub listen there.” **`arks hub`** stays the **`gh`-like client**. **`arks daemon`** stays git-daemon symmetry. **Default-all modules + `--no-*` opt-outs** keep clap lean; **fail-closed** keeps Partial SSH/forge honest. Prior “never put forge under serve” advice optimized noun symmetry; this version prioritizes **product unity**. Port numbers follow IANA/Git (**22 / 80 / 443 / 9418**) for the product story; unprivileged demos keep **8080 / 2222** labeled as alternates ([[research:root/20261111055609]]).

## Verification

Manual until hosting e2e exists:

1. `spec check` clean for this SoT and dependents.
2. Public `docs/self-host.md` matches umbrella model, default modules, fail-closed, config subcommand, auth split, hub-as-client, and **normative vs demo ports**.
3. Cross-links from product-intent / daemon / hub / public-surface / verify umbrella remain accurate.
4. No public Claimed for WAN-secure hosting, in-process SSHD, in-process TLS, forge-in-serve, or serve config without evidence.
5. Prose prefers `arks serve` / `--no-ssh` / `--no-http` / `--no-forge`; `serve-http` and `hub serve` labeled transitional only.
6. CLI `--help` for `arks serve` / `arks daemon` distinguishes normative ports from unprivileged demo binds.
