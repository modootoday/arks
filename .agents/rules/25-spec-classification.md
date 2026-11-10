# Spec classification (public repo)

Owning index: `.spec/README.md`. Process SoT: `.spec/sot/agent-governance.sot.md`. Public placement: `.spec/sot/public-surface.sot.md`.

## Must

- Put normative intent in `.spec/sot/`; plans in `.spec/plan/`; surveys in `.spec/research/`; scrubbed receipts in `.spec/evidence/`.
- Keep machine scratch, pin extracts, agent dumps, and anything secret-adjacent in **`.spec.local/`** (gitignored) — never promote into tracked `.spec/`.
- Keep `.agents/**` for rules/skills/role agents only — not product SoT dumps or evidence ledgers.
- Align active SoT / residual / tip tables with **single release = `arks`**: `git`→`arks`; `git-daemon`→`arks daemon` (internal `arks-daemon`); `gh`→`arks hub …` (internal `arks-hub`). Never Claim trimmed forge/host-env/LFS-product/CELIR-product rows; never advertise separate hub/daemon product bins.
- SoT body `Version **0.x.y**` is a **document revision**, not crates.io/`v*` product release (workspace still `0.0.0`). Bump only on normative meaning change; do not inflate on every design edit — rule `26-sot-version-vs-release`; see `.spec/README.md`.
- Spec meaning changes commit **before** related code — rule `27-spec-first-commit-order`.
- Mark historical research `status: archival` (or move to trash) when it would pollute active Claimed honesty — rule `28-evidence-honesty`.
- Assume the repo is or will be **public**; no secrets in tracked paths.
- **Do not push** while remote is empty / push-deferred unless the owner explicitly authorizes (rule `91-commit-hygiene`).

## Must not

- Commit `.spec.local/` or `.specfold/`.
- Re-Claim removed product surfaces (`arks forge`, `arks host-env`, in-process LFS product, CELIR as product Claimed path, OIDC/G8 live SLO as product DoD).
- Treat ops/CI harnesses (G8 live, OIDC release) as end-user Claimed product.
- Invent PASS/Claimed from research or chat alone.
- Publish or market a second CLI release (`arks-hub` / `arks-daemon` bins).
- Commit Hangul / non-English authored Spec or docs (rule `05-english-only`).

## Cite

- `.spec/README.md`
- `.spec/sot/agent-governance.sot.md`
- `.spec/sot/public-surface.sot.md`
- `.spec/sot/arks-hub-product-intent.sot.md`
- `.spec/sot/arks-daemon-product-intent.sot.md`
- `.spec/research/20261111034556-overdesign-candidates-exhaustive.research.md`
- `.spec/research/20261111035700-trim-overdesign-non-git.research.md`
