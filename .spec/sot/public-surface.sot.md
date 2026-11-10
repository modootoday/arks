---
id: sot:root/public-surface
title: Public surface
status: active
created: 2026-11-08T08:32:38Z
updated: 2026-11-10T19:45:00Z
tags: [public, docs, pages, license, security, agents, naming, honesty, alias, dot-git, perf, celir-index, g28, l3]
verify: manual
rel:
  depends_on:
    - sot:root/language-english-only
    - sot:root/product-naming
    - sot:root/agent-governance
    - sot:root/product-verification-clean-room
    - sot:root/git-compat-surface
    - sot:root/cli-surface
    - sot:root/program-roadmap
    - sot:root/clean-room-journey-compare
    - sot:root/production-residual-register
    - evidence:root/20261109212500
    - evidence:root/20261109213400
  governs:
    - docs/
    - README.md
    - SECURITY.md
    - .agents/rules/80-public-surface.md
---

# Public surface

Version **0.3.4**. Normative principles for what **arks** exposes when the repository is (or becomes) **public**, and what must remain non-public. Product naming: [[sot:root/product-naming]] (tagline **arks for Git**; working expansion **Agent Revision Kinetic System** at most once). Language: [[sot:root/language-english-only]]. Agent routing: [[sot:root/agent-governance]]. Compatibility claims in public copy MUST NOT contradict [[sot:root/git-compat-surface]] (same `.git`) or [[sot:root/cli-surface]] (CLI symmetry + alias drop-in). “Verified” claims MUST follow [[sot:root/product-verification-clean-room]]. Production wording MUST follow [[sot:root/program-roadmap]] (**CLI-parity production = M9**; **active alias bar = M16**; **L3 Claimed = M23**; **CELIR index dual-path = M24**; **M25** server transports G29–G32 Claimed; **M26** shared-index G34 Claimed; **G28** live-private journeys Claimed with receipts; residual honesty = [[sot:root/production-residual-register]]). Performance / “faster than Git” wording MUST follow [[sot:root/clean-room-journey-compare]] (clean-room receipts; prefer `.spec/evidence/`). Human Pages SHOULD lead with the multi-agent / concurrent-session contention motive from [[sot:root/product-intent]] without implying arks is an agent framework.

This SoT owns **public-vs-private placement**, **human-readable vs agent-governance vocabulary**, and **public-repo obligations**. It does not invent marketing copy, license body text, or Git semantics. Root SPDX identity is locked below to match `workspace.package.license`.

## Rule

### What is the public surface

1. The **public surface** is every path and artifact that is intended for anonymous or downstream consumers once `modootoday/arks` is public (and any already-published Pages site). It includes at least:
   - `docs/` — GitHub Pages site (`remote_theme: modootoday/jekyll-theme-modootoday`); human-facing product narrative.
   - Root `README.md` — first-contact summary, not a Specfold dump.
   - `LICENSE` (or equivalent SPDX-identified license file) — required before/at public release.
   - `CONTRIBUTING.md` — when contribution process is offered.
   - `SECURITY.md` (or linked security policy) — required at public release for vulnerability reporting.
   - `.github/CODEOWNERS` — ownership hints for reviews; not a substitute for SoT ownership.
   - **crates.io publish surface** — crate name `arks`, package metadata, crate README excerpts, and (later) docs.rs API docs for published crates.
2. Public surface copy MUST be **English only** ([[sot:root/language-english-only]]).
3. Tagline form on the public surface MUST follow the lock: **arks for Git** ([[sot:root/product-naming]]). The working expansion **Agent Revision Kinetic System** MAY appear **once** near first brand contact; afterward use short name `arks` and the tagline. Do not put a literal `git` substring in the crate, bin, or product short name.

### Human-readable vs agent-governance surfaces

4. Tracked paths fall into two audience lanes. Do not mix them casually:

| Lane | Paths (normative home) | Audience | Vocabulary |
|------|------------------------|----------|------------|
| **Human-readable** | `docs/`, root `README.md`, `LICENSE*`, `SECURITY.md`, `CONTRIBUTING.md` (when present), crates.io / docs.rs blurbs | Downstream humans | Plain product language: what arks is, how to try it, honesty limits. Prefer “clean-room receipt” / “evidence” over gate folklore dumps |
| **Agent-governance** | `AGENTS.md`, `.agents/**`, `.spec/**` (SoTs, plans, research, evidence catalogs), `verify/**` matrices and harness READMEs | Agents + maintainers | Specfold IDs, milestone/gate codes (M9, G14, G16, …), residual registers, claim-set ledgers — precise and non-duplicative |

5. **Human-readable** copy MUST NOT:
   - Dump Specfold process folklore (wave letters, claim-set IDs, residual plan filenames, chat process).
   - Overshare unfinished design, CELIR/lease internals, or raw evidence JSON.
   - Lead with gate catalogs (G0–G21 tables) — link to `verify/` or `.spec/` when detail is needed.
6. **Agent-governance** copy MUST:
   - Keep milestone/gate/evidence IDs precise and cross-linked.
   - Prefer updating owning SoTs over expanding `AGENTS.md` into a second ledger ([[sot:root/agent-governance]]).
   - Avoid restating humanymous mega ledgers; cite live paths under `C:\workspaces\humanymous`.
7. Root `README.md` is **human-first** but MAY include a short honest bar table with milestone names and evidence links. Deep residual disposition stays in [[sot:root/production-residual-register]].

### What must stay private / non-public

8. The following MUST NOT be committed to tracked public paths, published to Pages, or shipped in crates.io packages:

| Class | Examples | Normative handling |
|-------|----------|--------------------|
| Specfold local | `.spec.local/` | Already gitignored; never promote into tracked `.spec/` or `docs/` |
| Secrets / credentials | PATs, API keys, private keys, `.env`, cookies, CI secret values | Env/CI secrets only; scrub fixtures ([[sot:root/remote-host-compatibility-surface]]) |
| Private evidence raw | Unredacted remote traces, host logs with tokens, personal paths with credentials | Keep out of tree or redact before any commit |
| Unreleased differentiator internals | Invented CELIR APIs, unfinished lease wire, lab-only adapters | Boundary SoTs may stay in `.spec/` as **intent**; do not publish as shipped public API or Pages “how it works” until the owning SoT flips readiness ([[sot:root/storage-celir-boundary]], [[sot:root/lease-concurrency-intent]]) |
| Proprietary third-party | Non-redistributable vendor dumps, leaked partner docs | Forbidden in-tree |

9. Agents and humans MUST treat “will be public” as the default for every new tracked path: if it would be embarrassing or harmful on a public GitHub tree, do not add it.

### `.spec/` vs `docs/` (when the repo is public)

10. **`.spec/` is the product brain** — normative SoTs, plans, research, evidence catalogs. When the repository goes public, `.spec/` MAY become world-readable with the tree (see Public-flip decisions). It is **not** the default Pages content.
11. **`docs/` is the human-facing public narrative** — short product explanations, getting-started, and summaries suitable for GitHub Pages. It MUST NOT dump full SoT / research / plan trees by default.
12. Relationship:

| Concern | Belongs in `.spec/` | Belongs in `docs/` |
|---------|---------------------|--------------------|
| Normative MUST/MAY for implementers | Yes | No (link or one-line summary only) |
| Primary-source research catalogs | Yes | No (cite outcomes in prose if needed) |
| Product elevator + overview | Optional pointer | Yes |
| Milestone / gate ledgers | Yes (`program-roadmap`, residual register, matrices) | Thin honesty only; no folklore dumps |
| “Verified” claims | Evidence + clean-room receipts under Specfold | Only when a clean-room receipt exists; no host-only “verified” |
| Marketing fluff | No | Avoid; prefer accurate thin copy |

13. Publishing a SoT slice into Pages is allowed only when **intentional** (explicit owner decision or plan): summarize, do not paste large primary Git documentation or unfinished design ledgers.

### Future public-repo obligations

14. Before or at the flip from private → public, the repository MUST have (or an accepted plan that lands them in the same change set):
    - An open-source **license** at the root: SPDX **`Apache-2.0 OR MIT`** (dual; matches `Cargo.toml` `workspace.package.license`). Root `LICENSE` is the dual selector; full bodies are `LICENSE-APACHE` and `LICENSE-MIT`.
    - A **security policy** (`SECURITY.md`) describing how to report vulnerabilities (private GitHub Security Advisories; supported versions = `main` until a stable release policy exists).
    - English README that states product name, tagline **arks for Git**, and points at `docs/` / source without inventing Git wire semantics.
15. **CODEOWNERS** MAY be added under `.github/`; it is recommended for a multi-maintainer public tree but is not a hard blocker for going public.
16. **Trademark care:** Use tagline **arks for Git** as locked. Do not imply Git trademark ownership, official Git project affiliation, or “drop-in Git trademark” branding beyond accurate compatibility claims. Domain / trademark counsel remains required before aggressive public launch branding ([[sot:root/product-naming]]).
17. **No proprietary third-party leaks** — cite public primary docs by URL; do not vendor copyrighted manuals wholesale into `docs/` or README (summarize + link).
18. crates.io / docs.rs publish MUST use the locked crate name `arks`, English metadata, and MUST NOT embed secrets or private evidence in package contents.

### Compatibility honesty (normative)

19. Public copy MAY state the **end-state intent**: same on-disk `.git` layout ([[sot:root/git-compat-surface]]), Git CLI symmetric compatibility, and `git`→`arks` alias drop-in ([[sot:root/cli-surface]]) with native engine + clean-room verification.
20. Public copy MUST NOT claim that full CLI parity, alias drop-in readiness, CLI-parity **production**, or “drop-in `git` replacement today” is **shipped/verified** without the matching clean-room receipts ([[sot:root/program-roadmap]], [[sot:root/production-residual-register]]).
21. **Active alias honesty:** cite **M16** (complete flag symmetry — implement or N/A only) as the active alias bar. **M9** remains the historical CLI-parity inventory close; **M15** remains the historical inventory residual close. Do not present M15 refuse leftovers as the lasting production bar.
22. **Journey / performance honesty:** Public copy MUST NOT claim **performance superiority** (“faster than Git”, leaner, higher-throughput) as marketing. Comparative clean-room measurements (including M14/M17 / G19 allow-list work) MAY be linked as **evidence**; wording MUST match [[sot:root/clean-room-journey-compare]] honesty limits. Prefer linking `.spec/evidence/` over Pages slogans. Host-only timings MUST NOT appear as verified.
23. Lease (**M10**) and production-server (**M11**) wording MUST keep those bars separate from CLI-parity / alias claims and MUST match carrying receipts.
24. Do not imply official Git project affiliation or trademark ownership beyond accurate compatibility claims ([[sot:root/product-naming]]).

### Agent rules (normative)

25. Never write secrets, credentials, or unredacted private evidence into public paths (`docs/`, README, LICENSE-adjacent prose, CONTRIBUTING, SECURITY, crate READMEs, committed fixtures meant for public).
26. English only on the public surface (and on `.spec/` authored prose) — rule `.agents/rules/05-english-only.md` and `.agents/rules/80-public-surface.md`.
27. Do not contradict [[sot:root/git-compat-surface]] / [[sot:root/cli-surface]] in public compatibility claims; do not label host-only results as product-verified ([[sot:root/product-verification-clean-room]]).
28. Prefer updating this SoT over inventing chat-only “public docs policy.”
29. **CELIR honesty:** Public copy MAY say **CELIR dual-path Git verified** only with [[evidence:root/20261109205534]] (G23) and MUST state FS `.git` sole ObjectStore + CELIR index sidecar. Bare “CELIR-backed Git” / CELIR as object store remains forbidden ([[adr:root/20261109204916]]).
30. **L3 / shared-index honesty:** Public copy MAY say high-concurrency leases ship in the **claimed** L3 shape (lease + batch-apply + worktree/ref-shard) ([[adr:root/20261109194200]]). Shared-index N-way MAY be cited as **M26 / G34 Claimed** with [[evidence:root/20261110074200]]; MUST NOT conflate it with M23 L3 or with G22/G24 CELIR index gates ([[adr:root/20261110074000]]).
31. **Live forge / crypto honesty:** Public copy MAY cite **G28 Claimed** only with [[evidence:root/20261109212500]] (live) + [[evidence:root/20261109213400]] (offline clones; checklist N/A=0). MUST NOT imply forge throughput, Checks/merge-queue, or OpenPGP/`gpg.format=openpgp` / X.509 / `tag -s` support. **gpgsig** read + **SSH** create/verify MAY be cited with [[evidence:root/20261110073800]] (**G37**); OpenPGP remains N/A. G8 live smoke is secrets-default-on harness honesty — SKIP without secrets; never invent PASS. Tokens stay in env only.
32. **Partial residual honesty:** Public copy MUST keep filter/`blob:none`/`tree:0`/promisor **N/A**, forge OAuth **Refuse**, and Pf5 honesty current (**8/8** Claimed ≤0.5× on tip remasure [[evidence:root/20261110142500]]; do not resurrect CLONE-LOCAL NA). Credential fill (G33) MAY cite TTY + first-party OS wallets; MUST NOT market forge OAuth under CRED.

### Out of scope here

- Rewriting license body text (use upstream Apache-2.0 / MIT texts; do not invent terms).
- Writing full marketing site copy or visual brand system.
- Deciding whether humanymous / discrete-circuit private trees are ever vendored (they MUST NOT be; cite by path only when local).
- Implementing crates.io publish automation.
- Expanding `verify/` harness prose (owned by verification SoTs / matrices).

## Why

arks will eventually be a public repository. Without a canonical public-vs-private map and a human vs agent vocabulary split, agents dump Specfold into Pages, leave secrets near docs, or invent marketing that contradicts Git-compat and verification SoTs. One SoT keeps Pages thin, Specfold normative, and public-flip obligations explicit.

## Verification

Manual:

1. `spec check` clean for this document and linked assets.
2. Spot-check: no secrets under `docs/`, tracked `.spec/`, or root README/CONTRIBUTING/SECURITY when those files exist.
3. `docs/` does not contain a wholesale mirror of `.spec/sot/` or `.spec/research/`.
4. Public copy uses **arks** / **arks for Git** per naming lock (expansion at most once); English only.
5. Any “verified” claim on the public surface has a clean-room receipt path or is absent.
6. Any “CLI parity / alias drop-in / same `.git` / production / drop-in git replacement” claim matches roadmap milestone reality and residual register honesty, or is clearly marked as intent.
7. Human Pages do not dump Specfold process folklore; agent surfaces keep IDs precise without duplicating humanymous ledgers.
8. No “faster than Git” marketing without clean-room evidence + wording that matches journey-compare honesty.
9. `.gitignore` continues to exclude `.spec.local/` and common secret filename patterns listed there.

## Public-flip decisions (disposition)

| Decision | Disposition | Notes |
|----------|-------------|-------|
| Publish entire `.spec/` with the public repo? | **Closed (lean default)** | Default lean: **yes** (transparency). Owner may tighten at flip without reopening development debt. |
| Root LICENSE (SPDX) | **Done** | **`Apache-2.0 OR MIT`** — matches workspace Cargo license; `LICENSE` + `LICENSE-APACHE` + `LICENSE-MIT`. |
| `SECURITY.md` template depth | **Done** | Minimal: GitHub Security Advisories channel + pre-1.0 `main`-only supported versions; no secrets. |
| Root `CONTRIBUTING.md` | **Done** | Thin English pointer to `docs/contribute.md` + SECURITY/AGENTS; product-completion ladder L1 ([[research:root/20261110095142]]). |
| `.github/CODEOWNERS` | **Done** | Landed under `.github/CODEOWNERS` (`@modootoday`); ownership hints only. |
| Pages summaries of CELIR / lease | **Closed (honesty)** | Pages MUST stay high-level (“future storage / concurrency”) — no invented APIs until readiness SoT flips. |
| crates.io publish timing | **Closed (owner default)** | Name lock is not a reservation; publish/reserve before relying on exclusivity ([[sot:root/product-naming]]); timing remains operator choice, not a residual Open. Ladder **L9**. |
| Release-channel ops dry-run (OIDC Environments) | **Done (L4 ops)** | Environments `crates`/`npm`/`pypi`/`rubygems` + `release.yml` dry-run + `docs/release.md`; dry-run ≠ live registry publish PASS ([[research:root/20261110085731]], [[research:root/20261110095142]]). |
| Forge-scale ops dry-run (secrets-gated G8) | **Done (L5 ops)** | Environment `g8-live` + `g8-forge-scale.yml` dry-run + `docs/forge-scale.md`; dry-run ≠ live forge latency PASS ([[research:root/20261110095142]]). |
| Partial-clone filter/promisor residual (R3) | **Done (L6 honest N/A)** | SoT remains `--depth` only ([[sot:root/arks-remote]]); filter/`blob:none`/`tree:0`/promisor stay **N/A**; clone/fetch refuse `--filter`; **≠** partial-clone Claimed/PASS ([[research:root/20261110095142]]). |
| OpenPGP / X.509 / `tag -s` residual (R4) | **Done (L7 honest N/A)** | FW-GPGSIG stays SSH commit create/verify only ([[sot:root/arks-object]]); OpenPGP / X.509 / `tag -s` / `verify-tag` stay **N/A**; CLI refuses fail-closed; **≠** crypto Claimed/PASS ([[research:root/20261110095142]]). |
| Clean-room Docker multi-scenario e2e (L8) | **Done** | Sequential Docker PASS `g14`→`g16`(19/19)→`g20`→`g21` (`verify/matrices/matrix-l8-e2e.md` / `run-l8-e2e.sh`); host CI ≠ Verified; no invent PASS ([[research:root/20261110095142]]). |
| Git↔arks comparison matrix (L_final) | **In progress** | Public `docs/git-vs-arks.md` + Specfold residual/CLI honesty; Claimed/N/A/reopen-only only; never invent Claimed ([[research:root/20261110095142]]). |
| Whether `docs/` links into raw `.spec/` HTML on GitHub | **Closed (docs policy)** | Allowed as “source of truth on GitHub”; still do not Jekyll-publish full SoT trees by default. |
| Human vs agent vocabulary split | **Done (v0.3.0)** | Human Pages/README plain language; Specfold/AGENTS/`verify` keep precise IDs; no false “faster than Git” outside receipts. |
