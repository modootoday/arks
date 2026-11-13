# `.spec/` — public Specfold brain (arks)

This tree is **tracked** and will ship with the **public** repository. Treat every file as world-readable.

Remote is currently empty / push-deferred — **do not push** until the owner explicitly authorizes a clean history push.

## Product shape (owner Won — active canon)

**Language:** Specfold (`.spec/`) is English-only Source of Truth; chat/coordinator Korean briefings must not land in committed Spec or `docs/` ([[sot:root/language-english-only]]).

**Single release / single published CLI:** **`arks` only.**

Conceptual triad (stock peers) still guides design, but **shipping is one binary**:

| Concept | Stock peer | In the `arks` CLI | Internal crate (not a separate release) |
|---------|------------|-------------------|-----------------------------------------|
| Core Git | **`git`** | everyday porcelain/plumbing, remotes client, lease/race, measured perf | engine crates under `arks` |
| Daemon | **`git-daemon`** | **`arks daemon`** | **`arks-daemon`** (library; CLI front stays on `arks`) |
| Hub / forge | **`gh`** | **`arks hub …`** command group | **`arks-hub`** (library; no published `arks-hub` bin) |
| Agent bots | *(extension)* | **`arks agent …`** (review / CEV) | **`arks-agent`** (library; no published `arks-agent` bin) |

**Absolute:** product paths never spawn `git` or `gh`.

**Not product (trimmed / ops-only — do not Claim as core):** former `arks forge` thin-wrap, `host-env`, in-process LFS productization, CELIR as product Claimed path, OIDC release / G8 live SLO as product DoD. **Agent is an explicit extension** (like hub/daemon), not Git porcelain — design ≠ Claimed.

Over-design: [[research:root/20261111034556]], [[research:root/20261111034141]]. Trim: [[research:root/20261111035700]]. Hub: [[sot:root/arks-hub-product-intent]]. Daemon: [[sot:root/arks-daemon-product-intent]]. Self-host: [[sot:root/arks-self-host-server]]. Agent: [[sot:root/arks-agent-product-intent]].

## Classification map

| Class | Path | Public? | What belongs |
|-------|------|---------|--------------|
| **SoT** | `.spec/sot/*.sot.md` | Yes | Normative MUST/MAY. Active SoT must match single-`arks` release. |
| **Plan** | `.spec/plan/*.plan.md` | Yes | Executable slices that implement SoT. |
| **ADR** | `.spec/adr/*.adr.md` | Yes | Accepted architecture decisions. |
| **Research** | `.spec/research/*.research.md` | Yes | Surveys / inventories. Use `status: archival` when historical only. |
| **Evidence** | `.spec/evidence/<id>/` | Yes if scrubbed | Receipts, matrices, metrics — **no secrets**. |
| **Residual** | `.spec/sot/production-residual-register.sot.md` | Yes | Honesty ledger; no Claimed for trimmed surfaces. |
| **Trash** | `.specfold/trash/` | No (gitignored) | Specfold delete backups. |
| **Local-only** | `.spec.local/` | **Never** | Machine scratch, agent dumps, pin extracts. Gitignored. |

## `.spec/` vs `.spec.local/` vs `.agents/`

| Tree | Audience | Commit? |
|------|----------|---------|
| `.spec/` | Agents + maintainers; public transparency | **Yes** (scrubbed) |
| `.spec.local/` | This machine / this agent only | **Never** (`spec.toml` `[local] path`) |
| `.agents/` | Always-on rules, skills, role agents | **Yes** (no secrets) |
| `AGENTS.md` | Router only — not a second product SoT | **Yes** |

## How to add research

1. Create `.spec/research/YYYYMMDDHHMMSS-<slug>.research.md` with YAML frontmatter (`id`, `title`, `status`, `rel`).
2. Prefer `status: active` only if it still guides current SoT. Otherwise `status: archival`.
3. Do not invent **Claimed** / **PASS** from research alone — evidence + owning SoT amend required.
4. Link from owning SoT / residual / roadmap when it changes active honesty.

## SoT `Version` ≠ product release

Body line `Version **0.x.y**` on a SoT is a **document revision** (design-iteration counter), **not** the published `arks` crate/tag release.

| Signal | Where | Meaning today |
|--------|-------|----------------|
| Product / crate release | workspace `Cargo.toml` `version` + git tag `v*` | Still **`0.0.0`** / no public product release |
| SoT document revision | first body line `Version **…**` (+ frontmatter `updated:`) | Bumps when **normative meaning** of that SoT changes |
| Honesty class | prose + tags (`design`, `partial`) / residual rows | **Design ≠ Claimed**; status beats a high `0.x` number |

**Bump rules (agents):**

1. **Do not** bump SoT `Version` because code landed, cross-links were added, typos were fixed, or residual/tip tables were refreshed — use frontmatter `updated:` only.
2. **Do** bump only when MUST/MAY/inventory/advice **meaning** changes (breaking product story, supersession, new normative wave owned by that SoT). Prefer **+0.0.1** for additive clarifications, **+0.1.0** for superseding advice; stay in **0.x** until a deliberate post-release SoT policy says otherwise.
3. **Never** treat SoT `0.2 → 0.5` (or roadmap `0.15.x`) as “we shipped five releases.” Product release remains Cargo/tag only.
4. New design SoTs start at **0.1.0** and stay clearly marked **design / Partial** until Claimed + evidence.

## How Claimed works

- **Claimed** = product surface ships with carrying evidence (or explicit refuse/Impossible).
- **Ops/CI-only** = harness / workflows / maintainer docs — never end-user Claimed product.
- **Trimmed** = removed from product CLI/story; residual/roadmap MUST NOT keep Claimed language.
- **Design ≠ Claimed** — hub/daemon/agent design SoTs are intent until implemented + evidenced.
- **Do not publish** separate `arks-hub` / `arks-daemon` / `arks-agent` crates.io bins as product releases.
- Clean-room **Verified** requires Docker receipts; host greens are advisory.

## Commit order (history shape)

1. **Spec SoT canonization first** — land `.spec/sot/**` (+ this README / Spec classification cross-links) as its own commit(s) before any implementation.
2. **Then one step at a time** — CLI skeleton, stubs, verify harness, docs/viz — each as a **later** commit (or leave uncommitted until Spec tip is clean). Do **not** mix crates / workflows / HTML into a Spec canonization commit.
3. **SoT `Version` / document revision** = Spec edit counter only (**not** product release) until the first intentional product tag — see §SoT `Version` ≠ product release above.
4. Parallel agents: **do not commit code** until Spec-only tip exists; leave Rust/CLI/workflows dirty if Spec history is still being shaped.

## Secrets & push

- No secrets in `.spec/`. Evidence receipts OK when non-secret.
- Agent transcripts / local pin dumps → `.spec.local/` (or delete).
- **Push deferred** until owner authorizes ([[research:root/20261111041000]]). **No push / no force-push** unless the owner explicitly authorizes.

## Agent entry points

| Need | Read |
|------|------|
| Router | `/AGENTS.md` |
| Classification | this file + rule `25-spec-classification` |
| Process | [[sot:root/agent-governance]] (v0.3.2+ gap fill) |
| Governance gap matrix | [[research:root/20261111061037]] |
| Hangul / English-only | rule `05-english-only` + [[sot:root/language-english-only]] + `.cursor/hooks/*` |
| Spec-first / Version≠release / evidence / stub wording | rules `26`–`29`, `35` + skill `commit-governance` |
| Personality / serve≠hub / ports / no git-gh | rules `30`, `32`–`34` + skills `guard-cli-surface`, `review-product-subprocess`, `self-host-serve` |
| Symmetry / release gate / Cross-OS pin | rules `60`–`61` + skills `run-symmetry-matrix`, `release-symmetry-gate` |
| Research → SoT | skill `research-to-sot` |
| Public vs private | [[sot:root/public-surface]] |
| `arks` intent | [[sot:root/product-intent]] |
| Daemon (`arks daemon`) | [[sot:root/arks-daemon-product-intent]] |
| Product self-host (LAN/WAN) | [[sot:root/arks-self-host-server]] · public `docs/self-host.md` |
| Claimed residual gates (B1–B9) | [[sot:root/arks-claimed-residual-gates]] · [[research:root/20261111074523]] — design/SKIP harness; no invent Claimed |
| Remaining implement wave (research-gated) | [[sot:root/arks-remaining-implement-wave]] · [[research:root/20261111090827]] — IW-B5 / B7-SSH·forge Claimed; Partial leftovers → next row |
| Partial residual wave (research-gated) | [[sot:root/arks-partial-residual-wave]] · [[research:root/20261111093543]] — IW-FORGE-SERVE / HUB-FLAT / PERSONALITY / DAEMON-FLAGS; **do not invent Claimed** |
| Verify Docker e2e claim wave | [[sot:root/arks-verify-docker-e2e-claim-wave]] · cites [[research:root/20261111093543]] — hosting / hub STRICT subset / personality drop-in under `verify/`; B5+ Refuse |
| Remaining incompletes wave | [[sot:root/arks-remaining-incompletes-wave]] · [[research:root/20261111105924]] — hub v1 Missing→0 + expand; personality expand; daemon B5+ Refuse; docs honesty |
| Honesty leftovers wave | [[sot:root/arks-honesty-leftovers-wave]] · [[research:root/20261111114312]] — hub mutate subset; personality exit tables; B5+ collector reopen-or-Refuse; **no invent Claimed** |
| In-scope leftovers wave | [[sot:root/arks-in-scope-leftovers-wave]] · [[research:root/20261111123556]] — Phase 0 scope audit (git/daemon/gh + serve); Actions/assets/checkout/CLI Partial×117/daemon flags **in**; later/OUT/SysV/L9/G8 **out**; **no invent Claimed** |
| Optimized DoD renew (floor) | [[sot:root/arks-dod-optimized]] · [[research:root/20261113143029]] — Missing=0 + OD-* **floor only**; superseded as complete by behavioral DoD |
| Behavioral DoD renew | [[sot:root/arks-dod-behavioral]] · [[research:root/20261113194123]] — peer **actions**/exit/stdout; BP-PORCELAIN named set; hub terminal×6 Claimed-or-Refuse; Missing=0 alone ≠ complete; **no invent Claimed** |
| Verify directory conventions | [[sot:root/verify-directory-conventions]] · [[research:root/20261111082440]] — tree / folder naming / out / SKIP·STRICT / residual registration |
| Hub (`arks hub`) | [[sot:root/arks-hub-product-intent]] |
| CLI personality / factory | [[sot:root/arks-cli-personality-factory]] (git\|gh\|arks; OverlapFacade; ExitCodePolicy — design/Partial) |
| CLI output + exit contract | [[sot:root/arks-cli-output-exit-contract]] (stdout/stderr; porcelain flags; triple exit classes — design/Partial) |
| Agent (`arks agent`) | [[sot:root/arks-agent-product-intent]] · [[sot:root/arks-agent-cli-surface]] · [[sot:root/arks-agent-llm-providers]] · [[sot:root/arks-agent-cev-review]] |
| Residual | [[sot:root/production-residual-register]] |
| Tip currency | [[sot:root/program-roadmap]] tip table |

Before governed edits: `spec context <path>` → edit → `spec check`.

## Sibling ownership note

Clean-room **3-way symmetry verify matrices** (git↔arks, daemon, hub+multi-agent) live under umbrella [[sot:root/verify-cleanroom-symmetry-matrix]] + compose `dod-cli-matrix` / `dod-daemon-matrix` / `dod-hub-matrix` / `dod-symmetry` / `dod-symmetry-prerelease`. **Release precheck** [[sot:root/release-symmetry-precheck]] blocks tagged `dist` on pin Missing / untracked leaves (Partial + hub v1 Missing advisory by default). **Related hosting** [[sot:root/arks-self-host-server]] is **design-active** (LAN/WAN, arks-only); hosting e2e scaffold ≠ Claimed — do not invent PASS. **Adjacent Claimed residual** [[sot:root/arks-claimed-residual-gates]] (`dod-claimed-residual`) carries B1–B4/B7(HTTP)/B9 Claimed receipts where evidenced — **not** a fourth symmetry pillar; Partial smokes ≠ Claimed. **Remaining implement** Claimed rows (B5 / B7 SSH·forge) are gated by [[sot:root/arks-remaining-implement-wave]] + [[research:root/20261111090827]]. **Partial leftovers** (forge-in-serve hosting e2e, hub flat, personality drop-in, daemon flags/B5+) are gated by [[sot:root/arks-partial-residual-wave]] + [[research:root/20261111093543]]; Docker Claimed playbook [[sot:root/arks-verify-docker-e2e-claim-wave]]. **Post-sample incompletes** (hub full-matrix Missing→0, personality expand, B5+ terminal Refuse, docs lag) are gated by [[sot:root/arks-remaining-incompletes-wave]] + [[research:root/20261111105924]] — Spec-first; no invent Claimed. **Honesty leftovers** (hub mutate subset, personality per-cmd exit tables, B5+ collector reopen-or-Refuse) are gated by [[sot:root/arks-honesty-leftovers-wave]] + [[research:root/20261111114312]] — Spec-first; no invent Claimed. **In-scope leftovers** (Phase 0 pillar scope audit + Phase 1+ Actions/assets/checkout/CLI survey/daemon flag Parity) are gated by [[sot:root/arks-in-scope-leftovers-wave]] + [[research:root/20261111123556]] — Spec-first; OUT rows documented only; no invent Claimed. **Directory / naming consistency** for `verify/` is [[sot:root/verify-directory-conventions]] (ops/harness; not product Claimed). This tree owns **governance / classification / product-shape honesty**.
