---
id: research:root/20261108073228
title: GitHub-class remote hosting interoperability
status: active
created: 2026-11-08T07:32:28Z
updated: 2026-11-08T07:34:27Z
tags: [git, github, remote, hosting, interoperability]
rel:
  depends_on: [research:root/20261108071859, research:root/20261108071916]
---

## Question

Which public GitHub (and Git-protocol) documents define how a Git client must talk to github.com-class remotes for clone/fetch/push, and how that surface differs from GitHub product APIs (REST/GraphQL Apps, PRs, Checks)?

## Method

1. Reuse Git wire-protocol primary sources already catalogued in [[research:root/20261108071859]] and detailed in [[research:root/20261108071916]] (gitprotocol-http, gitprotocol-pack, gitprotocol-v2, capabilities, bundle-uri).
2. Enumerate GitHub Docs pages that describe remote URLs, HTTPS/SSH authentication for Git, and related hosting behavior.
3. Cross-check GitHub Blog / Changelog posts that state host-side protocol features (protocol v2, partial/shallow clone filters) when docs.github.com does not define wire bytes.
4. Explicitly separate **(A) Git wire/remote compatibility** from **(B) GitHub REST/GraphQL product APIs**.
5. Note GitLab/Bitbucket only as the same *class* of HTTPS/SSH Git remotes; do not treat their product APIs as normative for GIR MVP.
6. Do not invent host behavior; mark gaps where GitHub does not publish a claim.

Research snapshot date: **2026-11-08**.

## Authority tiers (this slice)

| Tier | Meaning |
|------|---------|
| **P0 — Git wire** | git-scm `gitprotocol-*` / `gitformat-*` (bytes and negotiation). |
| **G0 — GitHub Docs (host product)** | docs.github.com pages that state supported Git URL schemes, auth for Git, or Git LFS hosting rules. |
| **G1 — GitHub engineering posts** | github.blog / changelog posts announcing host-side protocol features; use for capability *expectations*, not as byte grammars. |
| **S — Secondary** | Other hosts’ docs, community write-ups; pointers only. |

## Findings

### Scope map: (A) Git wire vs (B) product APIs

| Layer | What it is | MVP for “acts as git remote client” | MVP for “acts as git remote server” | Notes |
|-------|------------|--------------------------------------|--------------------------------------|-------|
| **(A) Git wire / remote** | Smart HTTPS + SSH pack/v2 sessions; capability negotiation; object/ref transfer | **Critical** | **Open / phased** (see SoT) | Normative bytes in gitprotocol-*; host URL/auth in GitHub Docs |
| **(A′) Host Git extensions** | Git LFS pointer + separate LFS transfer; file-size limits | **Phased** (interop with LFS remotes) | Phased | Documented by GitHub; not core gitprotocol |
| **(B) GitHub product APIs** | REST/GraphQL: PRs, Checks, Apps, Issues, Actions metadata | **Non-goal for Git-remote MVP** | N/A | Separate product surface; may be later SoT |

### A. Git wire primary sources (P0) — already owned by protocols research

Summaries live in [[research:root/20261108071916]]. For GitHub-class remotes the critical bindings are:

| Doc | URL | Hosting relevance |
|-----|-----|-------------------|
| gitprotocol-http | https://git-scm.com/docs/gitprotocol-http | Dominant transport for `https://github.com/...` remotes (smart HTTP) |
| gitprotocol-pack | https://git-scm.com/docs/gitprotocol-pack | Fetch/push dialog shared by SSH and classic pack path |
| gitprotocol-v2 | https://git-scm.com/docs/gitprotocol-v2 | Modern capability/command model; HTTP `Git-Protocol: version=2` |
| gitprotocol-capabilities | https://git-scm.com/docs/gitprotocol-capabilities | v1 capability names; never invent |
| bundle-uri | https://git-scm.com/docs/bundle-uri | Optional v2 seed mechanism **when advertised** |
| packfile-uri | https://git-scm.com/docs/packfile-uri | Optional pack CDN URI mechanism **when advertised** |
| partial-clone | https://git-scm.com/docs/partial-clone | Filter / promisor semantics |

### B. GitHub Docs — remotes, transports, auth (G0)

| Doc | URL | Scope (summarized) | Why for GIR |
|-----|-----|--------------------|-------------|
| About remote repositories | https://docs.github.com/en/get-started/git-basics/about-remote-repositories | Push URLs are **HTTPS** (`https://github.com/user/repo.git`) or **SSH** (`git@github.com:user/repo.git`); clone/fetch/pull/push over those URLs | Defines the GitHub remote URL surface clients must accept |
| About authentication to GitHub | https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/about-authentication-to-github | HTTPS: PAT (password auth for Git removed) or GitHub CLI; SSH: keypair; credential helpers | High-level auth patterns for Git over HTTPS/SSH (no secrets in design) |
| About SSH | https://docs.github.com/en/authentication/connecting-to-github-with-ssh/about-ssh | SSH keys for Git data access and commit signing | SSH client expectations toward github.com |
| Using SSH over the HTTPS port | https://docs.github.com/en/authentication/troubleshooting-ssh/using-ssh-over-the-https-port | Optional `ssh.github.com:443` when port 22 blocked; not supported on GHES / data-residency per that page | Firewall-friendly SSH path |
| Set up Git | https://docs.github.com/en/get-started/git-basics/set-up-git | HTTPS recommended; credential caching; SSH key setup pointers | Onboarding path users expect |
| Managing remote repositories | https://docs.github.com/en/get-started/git-basics/managing-remote-repositories | `git remote set-url` between HTTPS and SSH forms | URL shape interchange |
| Getting changes from a remote | https://docs.github.com/en/get-started/using-git/getting-changes-from-a-remote-repository | clone / fetch / pull / merge against GitHub remotes | User-visible client operations |
| Cloning a repository | https://docs.github.com/en/repositories/creating-and-managing-repositories/cloning-a-repository | HTTPS, SSH, and GitHub CLI clone entry points | Host-published clone URLs |
| Managing your personal access tokens | https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/creating-a-personal-access-token | PATs as HTTPS Git password substitute | Auth token class (never store secrets in specs) |
| Caching your GitHub credentials in Git | https://docs.github.com/en/get-started/git-basics/caching-your-github-credentials-in-git | Credential helpers / Git Credential Manager | Client credential UX |
| GitHub’s SSH key fingerprints | https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/githubs-ssh-key-fingerprints | Published host key fingerprints | SSH trust anchors |

Auth patterns at design level (no secrets):

- **HTTPS smart HTTP:** username + PAT (or helper-provided token); Basic auth style as expected by gitprotocol-http SHOULD for clients; password login for Git is removed per GitHub Docs.
- **SSH:** user `git`, public-key auth; optional port-443 hop via `ssh.github.com`.
- **SAML SSO orgs:** PAT/SSH key may need org authorization (linked from About remote repositories) — host policy, not wire grammar.

### C. GitHub-stated protocol capabilities (G1 — expectations, not byte SoT)

docs.github.com does **not** publish a full capability advertisement matrix. The following are **GitHub-published** statements about host behavior; wire details remain P0 gitprotocol docs.

| Claim | Source | URL | Design takeaway |
|-------|--------|-----|-----------------|
| GitHub supports Git wire protocol v2 | Changelog | https://github.blog/changelog/2018-11-07-git-protocol-v2-support/ | Client SHOULD speak v2 toward github.com; still tolerate v1 |
| On github.com / GHES 2.22+: partial clone filters `blob:none` and `tree:0`; also shallow `--depth` | Engineering blog | https://github.blog/open-source/git/get-up-to-speed-with-partial-clone-and-shallow-clone/ | When GIR claims partial/shallow client support against GitHub, exercise these filters; server may deny other filters |
| Shallow clones discouraged for ongoing developer fetch (stress / semantics) | Same blog | same | Prefer full or blobless partial for durable workspaces |

**bundle-uri / packfile-uri on github.com:** No docs.github.com page found (snapshot date) that states github.com advertises `bundle-uri` for clones. Git project defines the mechanism (P0). GitHub Blog discusses the *Git feature* (e.g. Git 2.50 highlights) without a clear “github.com always advertises bundle-uri” product claim. **Treat as: negotiate only if advertised; do not assume GitHub advertises it.**

### D. Git LFS (host extension adjacent to remotes)

| Doc | URL | Scope |
|-----|-----|-------|
| About Git Large File Storage | https://docs.github.com/en/repositories/working-with-files/managing-large-files/about-git-large-file-storage | Pointer files in Git; content stored separately; plan size limits |
| About large files on GitHub | https://docs.github.com/en/repositories/working-with-files/managing-large-files/about-large-files-on-github | Soft/hard Git blob size limits; LFS for oversized files |

LFS is **not** gitprotocol; it is a common GitHub-class remote companion. Catalog for phased interop; do not conflate with pack/v2 MVP.

### E. (B) GitHub product APIs — in design map, not Git-remote MVP

| Doc | URL | Role |
|-----|-----|------|
| REST API overview | https://docs.github.com/en/rest | Product HTTP API (`api.github.com`), not git upload-pack |
| GraphQL API | https://docs.github.com/en/graphql | Same product plane |
| Pulls (GraphQL) | https://docs.github.com/en/graphql/reference/pulls | PR metadata |
| Checks / check runs (REST) | https://docs.github.com/en/rest/checks/runs | CI check runs (Apps write) |
| Building CI checks with a GitHub App | https://docs.github.com/en/apps/creating-github-apps/writing-code-for-a-github-app/building-ci-checks-with-a-github-app | App/Checks product workflow |

These define **forge product** behavior. GIR may later consume them for PR/Checks UX; they are **orthogonal** to speaking Git to github.com as `origin`.

### F. Same-class remotes (secondary)

GitLab and Bitbucket also expose HTTPS and SSH Git remotes using the same Git smart protocols. Useful as regression peers once GitHub-class client tests exist; their REST/GraphQL/MR APIs are separate product surfaces and are **not** normative for this SoT. Cite only if/when a dedicated research slice is opened.

### Capability expectations for GIR as GitHub remote **client**

Standards-first: implement negotiation per P0; host expectations from G0/G1:

1. **HTTPS smart HTTP** clone/fetch/push to `https://github.com/...` (and enterprise hosts of the same class when configured).
2. **SSH** clone/fetch/push to `git@github.com:...` (and optional `:443` / `ssh.github.com` when needed).
3. **Auth:** PAT/helper over HTTPS; SSH keys over SSH — integrate with credential/SSH agent patterns; never embed secrets in specs or fixtures committed to the repo.
4. **Protocol v2** preferred; fall back per Git rules when peer does not speak v2.
5. **Shallow / partial:** client MUST only request filters the session negotiated; against GitHub, documented host filters are `blob:none` and `tree:0` (G1). Do not invent filter specs as “GitHub-supported” without a citation.
6. **bundle-uri:** consume if advertised; do not require for MVP success against github.com until a public host claim exists.
7. **LFS:** phased; pointer-aware Git object store + LFS batch API is a later claim.

### Explicit gaps (do not invent)

- Exact live capability advertisement strings from github.com (agent, filter, object-format, …) — measure in evidence later; not asserted here.
- Whether github.com currently advertises `bundle-uri` / packfile-uri.
- Full GitHub Enterprise Server vs github.com deltas beyond what cited pages already state (e.g. SSH-over-443 unsupported on GHES).

## Open follow-ups

- Evidence run: record capability advertisement from a public github.com repo (HTTPS + SSH) into `.spec` evidence — without committing secrets.
- Pin whether GIR **server** role targeting stock `git` / GitHub Actions `actions/checkout`-class clients is MVP, later phase, or out of product scope ([[sot:root/remote-host-compatibility-surface]]).
- Separate SoT if/when GIR adopts GitHub Apps / REST / GraphQL for PR/Checks.
