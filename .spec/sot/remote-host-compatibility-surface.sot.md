---
id: sot:root/remote-host-compatibility-surface
title: Remote host compatibility surface
status: active
created: 2026-11-08T07:32:28Z
updated: 2026-11-08T10:54:46Z
tags: [git, github, remote, compatibility, hosting, server, smart-http, ssh]
verify: [manual]
rel:
  depends_on: [sot:root/git-compat-surface, sot:root/cli-surface, sot:root/program-roadmap, research:root/20261108073228, research:root/20261108071916, research:root/20261108071859]
  evidence: [research:root/20261108073228, research:root/20261108071916]
---

# Remote host compatibility surface

Version **0.2.0**. Normative intent for arks interoperability as a Git **remote client** and as a Git **server**, plus the (A)/(B) host-product split. Wire bytes remain owned by [[sot:root/git-compat-surface]] and [[research:root/20261108071916]]; this SoT owns **remote/host roles**, verify-peer policy, and server-role lock. Host citation catalog: [[research:root/20261108073228]]. Program bars: [[sot:root/program-roadmap]] (**M9** CLI-parity client production; **M11** production-server).

## Rule

### Scope split (normative)

1. **(A) Git wire / remote compatibility** — Speaking documented Git protocols (smart HTTPS, SSH pack/v2, and other claimed transports) so clone/fetch/push interoperate. Covers arks as **client** (any Git remote speaking the claimed protocols) and arks as **server** (stock Git / arks clients speaking those protocols to arks).
2. **(B) Host product APIs** — GitHub REST/GraphQL (PRs, Checks, Apps, Issues, Actions metadata, etc.). These remain **design-mapped** but **out of** CLI-parity production and **out of** production-server. A later SoT may adopt them; inventing forge semantics is forbidden here.

### Client remotes — generic protocols + GitHub-class verify peer

3. **Generic client goal.** When remote-client code ships, arks MUST interoperate for **clone / fetch / push** (and pull forms as claimed) with **any Git remote** that speaks the **claimed** transports and capabilities — not GitHub-only. Claimed wire behavior cites [[sot:root/git-compat-surface]] / [[research:root/20261108071916]] (gitprotocol-http, gitprotocol-pack, gitprotocol-v2, SSH pack invocation).
4. **GitHub-class = representative verify host.** github.com (and hosts that expose the same HTTPS/SSH Git URL + smart-protocol surface) is the **primary clean-room / live-smoke peer** for client remotes (gates **G7**/**G8**/**G9**), not the exclusive product remote type. Same-class remotes (GitLab/Bitbucket-style HTTPS/SSH Git) are welcome regression peers for (A).
5. **Git bytes from Git project docs.** Capability names, pkt-line framing, HTTP service paths, and pack transfer MUST follow primary sources catalogued in [[research:root/20261108071859]] / [[research:root/20261108071916]]. Do not invent capabilities.
6. **Host URL and auth from public host docs when the peer is a named host.** github.com remote URL schemes and Git authentication patterns MUST follow GitHub Docs cited in [[research:root/20261108073228]] (HTTPS + PAT/helper; SSH keys; optional SSH-over-443). Do not invent host auth schemes. For generic remotes, use documented Git credential-helper / SSH identity patterns — not forge OAuth UX.
7. **Host capability claims need citations.** Statements that “GitHub supports X” MUST cite a GitHub Docs or GitHub Blog/Changelog source from [[research:root/20261108073228]] — or be labeled unverified and measured as evidence. Prefer negotiation: only use what the peer advertises.
8. **Secrets stay out of the tree.** Specs, fixtures, and committed tests MUST NOT contain PATs, private keys, or session cookies. Auth in tests uses env/CI secrets or recorded scrubbed fixtures.

### Client-to-remote (required before CLI-parity production — M6 → M9)

When arks implements remote client operations, it MUST:

| Concern | Normative intent | Citations |
|---------|------------------|-----------|
| HTTPS smart HTTP | Interoperate for clone/fetch/push with `https://…` remotes using smart HTTP (GitHub-shaped URLs are the verify peer, not the only shape) | gitprotocol-http; GitHub About remote repositories (verify peer) |
| SSH | Interoperate for clone/fetch/push with `git@host:path` / `ssh://` as claimed (GitHub forms are the verify peer) | gitprotocol-pack; GitHub About SSH / SSH-over-HTTPS port (verify peer) |
| Auth (high level) | HTTPS: credential helper / token-as-password patterns as documented; SSH: key-based auth via agent/configured identity | Git credential / SSH docs; GitHub auth docs for verify peer |
| Capability negotiation | Prefer protocol v2 when peer offers it; never advertise unimplemented caps; ignore unknown optional peer caps per Git docs | gitprotocol-v2 / capabilities |
| Shallow / partial | May request shallow/partial only per negotiated capabilities; against GitHub verify peer, documented host filters include `blob:none` and `tree:0` — do not claim other host filters without citation | partial-clone; GitHub partial/shallow blog |
| bundle-uri | MAY consume bundle-uri if advertised; MUST NOT require github.com to advertise it for client pass | bundle-uri; research gap note |
| LFS | Git LFS is phased companion interop, not a substitute for pack/v2 correctness | GitHub Git LFS docs (when claimed) |

### Server role — **LOCKED** (in-scope product goal; **M11** production-server)

9. **Decision status: locked in.** arks **SHALL** implement a Git **server** role so stock Git clients (and arks-as-client) can **clone / fetch / push** against an arks-hosted repository for the **claimed** subset. This is a **first-class implementation goal** ([[sot:root/product-intent]]) with program exit at **M11** ([[sot:root/program-roadmap]]). It is **not** part of CLI-parity production (**M9**).
10. **Server services (claimed subset).** Production-server MUST implement documented **upload-pack** (fetch/clone path) and **receive-pack** (push path) semantics for the advertised capability set — cite [gitprotocol-pack](https://git-scm.com/docs/gitprotocol-pack), [git-upload-pack](https://git-scm.com/docs/git-upload-pack), [git-receive-pack](https://git-scm.com/docs/git-receive-pack). Protocol v2 commands when advertised MUST follow [gitprotocol-v2](https://git-scm.com/docs/gitprotocol-v2). Never advertise unimplemented capabilities; refuse unknown/unimplemented extensions per [[sot:root/git-compat-surface]].
11. **Transports (server).**

| Transport | Production-server default | Primary docs |
|-----------|---------------------------|--------------|
| Smart HTTP (`info/refs` + service POST) | **Required** for M11 exit when server ships | [gitprotocol-http](https://git-scm.com/docs/gitprotocol-http), [git-http-backend](https://git-scm.com/docs/git-http-backend) |
| SSH (invoke upload-pack / receive-pack) | **MAY** claim in M11 or later amendment; required only if advertised | gitprotocol-pack; upload-pack / receive-pack man pages |
| `git://` daemon | **MAY** claim; not required for first production-server bar | [git-daemon](https://git-scm.com/docs/git-daemon) |
| Dumb HTTP | **MAY** omit; if omitted, docs MUST NOT claim dumb | gitprotocol-http |

12. **Auth surface (server, high level).** HTTP: documented Basic / reverse-proxy / front-door auth patterns appropriate to smart HTTP — do not invent forge session cookies or GitHub App installation tokens **as** the arks server identity model. SSH (when claimed): key-based auth and command restriction in the spirit of `git-shell` / forced-command upload-pack|receive-pack. Secrets never in tree.
13. **Hooks.** When receive-pack / upload-pack would run server-side hooks under stock Git for the claimed path, honor documented [githooks](https://git-scm.com/docs/githooks) points for that subset (or refuse with clear message if a hook class is not yet claimed). Do not invent a parallel hook API.
14. **Out of server scope (normative non-goals).**

| Out | Rationale |
|-----|-----------|
| Forge UX / web UI | Product non-goal; scope (B) |
| GitHub App / Checks / PR merge **as server** | Scope (B); arks is not a GitHub-compatible forge |
| Emulating GitHub permissions / billing | Scope (B) |
| Requiring CELIR | [[sot:root/storage-celir-boundary]] |
| Lease wire exposure | Until [[sot:root/lease-concurrency-intent]] explicitly claims it |
| Invented protocol capabilities | [[sot:root/git-compat-surface]] |

15. **Phasing / honesty.** Public docs and README MUST NOT claim production-server, mirror-to-GitHub-as-forge, or “GitHub can push to arks as a first-class forge target” until **M11** + **G15** receipts exist. Optional later integration check: mirror-style push into a GitHub empty repo remains (A), not (B).

### Goals vs non-goals

**Goals (this SoT)**

- arks as Git **client** interoperates with **any** claimed-protocol remote; GitHub-class hosts are the **representative verify peer**.
- arks as Git **server** interoperates with stock Git clients for the claimed upload-pack / receive-pack + transport subset (**M11**).
- Capability-safe negotiation; standards-first wire behavior; clear (A)/(B) boundary.

**Non-goals (this SoT)**

- Implementing GitHub Apps, REST, or GraphQL for PRs, Checks, Issues, Projects, Actions orchestration.
- Emulating GitHub’s web UI, permissions product model, or billing.
- Claiming bundle-uri, arbitrary filters, or LFS completeness without citations + tests.
- Inventing GitHub or Git wire behavior not present in public docs or measured evidence.
- Folding server into **M9** CLI-parity production.

### Test plan sketch (plan only — no harness required yet)

Authoritative execution is inside the Docker clean-room ([[sot:root/product-verification-clean-room]]; matrix [[sot:root/interop-test-matrix]] / [[research:root/20261108073650]]).

**Client (M6+)**

1. Live or CI round-trip against a dedicated throwaway **GitHub-class** repo (verify peer) — clone, fetch, push; scrub logs.
2. Generic-remote regression when fixtures exist (any smart-HTTP/SSH peer speaking claimed protocols).
3. Partial/shallow matrix when claimed; transport matrix HTTPS + SSH; recorded fixtures offline (**G7**).
4. Negative cases: unknown capability; auth failure; refuse incomplete claims.

**Server (M11 — G15)**

5. Stock `git clone` / `fetch` / `push` from the clean-room Git pin against arks server for each **claimed** transport.
6. Capability refuse tests: arks MUST NOT advertise unimplemented caps; clients must still complete baseline clone/fetch/push for the claimed subset.
7. Optional later: mirror-style push into a GitHub empty repo as an integration check (still (A)).

Baseline Git client version for fixtures: align with the clean-room Git pin under [[sot:root/product-verification-clean-room]] / [[sot:root/git-compat-surface]].

## Why

Real workflows need clone/fetch/push to **ordinary Git remotes**, with GitHub-class hosts as the hardest common verify peer — not as the only peer. Separately, acting as a Git **server** (upload-pack / receive-pack) is part of “arks for Git” infrastructure without becoming a forge. Keeping (A) wire roles distinct from (B) product APIs, and keeping **M11** off the **M9** client production bar, preserves honesty.

## Verification

Manual until harness exists. Product “verified” for remote client/server claims requires clean-room success per [[sot:root/product-verification-clean-room]].

1. `spec check` clean for this SoT and [[research:root/20261108073228]] / [[research:root/20261108071916]].
2. Every host-behavior claim cites a URL from research (or is marked open/evidence-needed).
3. Implementation PRs that touch remotes list client vs server claims and which test sketch / Gate IDs they cover.
4. No committed secrets; fixture redaction reviewed.
5. Server-role marketing requires **M11** / **G15** — not M6/M9 alone.
