# Public surface (will-be-public repo)

arks will be a **public** repository. Treat every new tracked path as world-readable. Owning SoT: `.spec/sot/public-surface.sot.md`.

## Must

- Keep secrets, credentials, `.env`, PATs, private keys, and unredacted private evidence **out** of tracked paths (`docs/`, README, CONTRIBUTING, SECURITY, crate metadata, fixtures).
- Prefer thin `docs/` (human narrative) over dumping `.spec/` SoTs/research into GitHub Pages.
- Separate audiences: **human-readable** (`docs/`, README, LICENSE, SECURITY) uses plain product language; **agent-governance** (`AGENTS.md`, `.agents/`, `.spec/`, `verify/` matrices) keeps Specfold IDs precise.
- English only on the public surface (rule `05-english-only`).
- Use product lock **arks** / tagline **arks for Git**; no literal `git` in crate/bin/product short name.
- Do not label host-only results as product-verified (rule `60-clean-room-verify`).
- Cite **M16** as the active alias bar; do not market “faster than Git” without clean-room evidence that matches journey-compare honesty.
- **CELIR honesty:** MAY say dual-path index verified only with G23 receipt and MUST state FS `.git` sole object store + optional `celir-index` sidecar. Bare “CELIR-backed Git” / CELIR as object store is forbidden.
- **L3 honesty:** Claimed shape is lease + batch-apply + worktree/ref-shard; shared-index N-way is a separate **M26 / G34** claim (do not conflate with L3 or CELIR G22/G24).
- **Partial honesty:** filter/promisor N/A; Pf5 partial 7/8 (CLONE-LOCAL NA); OpenPGP N/A; OAuth Refuse; credential fill Claimed includes TTY + OS wallets (G33) — no OAuth marketing.
- **Live forge journeys:** cite only with receipts; G8 secrets-default-on harness may SKIP without secrets — never invent PASS; tokens stay in env only.

## Must not

- Commit `.spec.local/` or promote local Specfold state into public trees.
- Invent CELIR/lease public APIs in Pages or crate docs before owning SoTs flip readiness.
- Enable or advertise ObjectStore-via-CELIR.
- Paste large copyrighted primary Git manuals into `docs/` or README (summarize + link).
- Dump Specfold process folklore (wave letters, claim-set IDs, residual plan filenames) into human Pages.
- Imply Git trademark ownership or official Git project affiliation.

## Cite

- SoT: `.spec/sot/public-surface.sot.md`
- Naming: `.spec/sot/product-naming.sot.md`
- Language: `.spec/sot/language-english-only.sot.md`
- Verification: `.spec/sot/product-verification-clean-room.sot.md`
- Residual honesty: `.spec/sot/production-residual-register.sot.md`
- CELIR dual-path: `.spec/sot/celir-index-dual-path.sot.md`

If public-vs-private placement is unclear: **stop and ask** (rule `40-ambiguity-ask`).
