---
id: research:root/20261111001816
title: Partial completion — per-gap unblock roadmap
status: active
created: 2026-11-10T15:18:16Z
updated: 2026-11-10T15:34:44Z
tags:
  [
    partial-completion,
    unblock,
    roadmap,
    fs-final,
    g8-live,
    l9,
    pf5,
    g19,
    obj-004,
    fs-007,
    fs-014,
    win-003,
    forge-ux,
    non-goals,
  ]
verify: manual
rel:
  depends_on:
    - sot:root/program-roadmap
    - sot:root/production-residual-register
    - sot:root/public-surface
    - research:root/20261110225000
    - research:root/20261110222000
    - research:root/20261110211804
    - research:root/20261110212414
    - evidence:root/20261110224814
    - adr:root/20261110220000
    - adr:root/20261110162000
---

# Partial completion — per-gap unblock roadmap

## Question

After tip ≥ **`60f33c2`** (clean-room matrix currency ≥ `54c9dc2`), with **FS_final YES** but overall operator honesty **PARTIAL**, what is each remaining gap’s status, blocker class, concrete unblock options (A/B), owner/secrets/FFI deps, recommended sequence, and risks — without inventing PASS, force-publishing crates, or stampeding G19?

## Method

1. Reconcile tip currency against [[sot:root/program-roadmap]] v0.15.2 tip table, [[sot:root/production-residual-register]], [[research:root/20261110225000]], [[evidence:root/20261110224814]].
2. Fold prior unblock research ([[research:root/20261110222000]], [[research:root/20261110211804]]) — note what already landed (sha1cd detect+refuse, host-env advisory).
3. Re-probe Environment `g8-live` secret **names only** (`total_count`).
4. Rank honest next actions. No invent Claimed/PASS. No L9 publish. No G19 remasure thrash.

## Tip honesty baseline

| Item | Value |
|------|-------|
| Tip (this note) | ≥ `307d509` + U-ladder [[evidence:root/20261111003444]] |
| FS_final git feature DoD | **YES** |
| Operator one-word | **PARTIAL** |
| G19 M17 ≤0.8× | **19/19 PASS** |
| Pf5 ≤0.5× tip | U1 one amortize wave → sole remasure; else **Claimed(partial)** |
| `g8-live` secret count | **0** (re-probed 2026-11-11) |
| L9 | Owner-closed — reopen pack [[research:root/20261111003444]] |
| Forge UX | Claimed-closest `arks forge` thin-wrap (was Non-goal) |
| Host-env | Claimed detect+opt-in refuse / FS-014 LSM detect |

## U-ladder (2026-11-11)

| Step | Result |
|------|--------|
| U1 Pf5 | Amortize path landed; one sole G19 only |
| U2 host-env | Deepened FS-007/WIN-003/FS-014 |
| U3 live G8 | Secrets absent → local-forge wire; no invent PASS |
| U4 L9 | Reopen pack only; no publish |
| U5 forge | `arks forge` Claimed-closest |
| U6 sha1cd | Immunity Impossible (docs) |

---

## Per-gap ledger

### 1. Live G8 latency SLO (secrets)

| Field | Content |
|-------|---------|
| **Status** | **Blocked** — harness Claimed / default-on when secrets; live remasure SKIP |
| **Blocker class** | Operator secrets (Environment `g8-live`) |
| **Deps** | Owner/operator PAT + throwaway HTTPS URL; Actions or compose; evidence attach |
| **Option A (recommended)** | Fill `ARKS_G8_TOKEN` + `ARKS_G8_REPO_URL` in Environment `g8-live` → `gh workflow run g8-forge-scale.yml -f dry_run=false` → attach receipts under `.spec/evidence/` → update residual R5 |
| **Option B** | Local compose `g8` / `g8-live-scale` with same env names exported; same receipt honesty |
| **Not options** | Invent medians; treat SKIP / local-forge `claimed-wire` as GitHub live PASS |
| **Risks** | PAT leak in logs; colliding sole-runner Docker; marketing forge throughput from smoke latency |

### 2. L9 crates.io / docs.rs timing (owner)

| Field | Content |
|-------|---------|
| **Status** | **Blocked** — owner policy closed |
| **Blocker class** | Owner SoT / publish policy |
| **Deps** | Owner decision; `publish = true` flip; OIDC trusted publisher / Environment `crates` |
| **Option A (recommended)** | Wait for owner SoT reopen; then dry-run → tag → trusted publish with docs.rs follow |
| **Option B** | Keep closed; document release readiness only (`docs/release.md`) |
| **Not options** | Unauthorized `cargo publish`; agent invents L9 Done |
| **Risks** | Premature public crate API freeze; yank churn |

### 3. SHA-1 collision **immunity** (crypto) vs detect+refuse

| Field | Content |
|-------|---------|
| **Status** | Detect+refuse **Done** (Claimed); immunity **Non-goal** |
| **Blocker class** | Cryptographic Impossible (math claim) |
| **Deps** | None for immunity; detect path already shipped (`sha1collisiondetection` / sha1cd) |
| **Option A (recommended)** | Keep wording: **detect+refuse Claimed**; never market “immune” |
| **Option B** | Optional further ingest-path coverage audit + SHAttered fixture regress only |
| **Risks** | Agents reopen “immunity” as development Open; G19 thrash after hasher change |

### 4. Forge Checks / PR / Issues / OAuth UX (not-git)

| Field | Content |
|-------|---------|
| **Status** | **Non-goal** |
| **Blocker class** | Product scope (revision layer ≠ forge host UX) |
| **Deps** | Primary SoT reopen if arks becomes forge-client product |
| **Option A (recommended)** | Docs “use `gh`”; honor external `credential.helper` (already Claimed) |
| **Option B** | Optional thin `gh` recipe appendix — still not Claimed forge UX |
| **Risks** | Scope creep into Checks/merge-queue product; OAuth App legal/client registration |

### 5. FS-007 full cloud-sync safety

| Field | Content |
|-------|---------|
| **Status** | **Partial** — advisory detect useful (DriveFS G:); full multi-device safety **Non-goal / Impossible** |
| **Blocker class** | Host multi-device physics; cannot package away sync races |
| **Deps** | Optional SoT amend for hard-refuse flag; advisory harness |
| **Option A (recommended)** | Deepen detect + optional config refuse when `.git` under sync root; keep `claimed=false` for multi-device |
| **Option B** | Out-of-band two-machine protocol research only — still unlikely Claimed |
| **Risks** | Promoting G: advisory smoke to Claimed FS-007; conflating DriveFS with OneDrive |

### 6. WIN-003 OneDrive hydration

| Field | Content |
|-------|---------|
| **Status** | **Partial** — advisory CfAPI / `RECALL_ON_DATA_ACCESS` detect; hydration implement Impossible |
| **Blocker class** | Host OS placeholder semantics; needs real OneDrive tree (G: DriveFS ≠ WIN-003) |
| **Deps** | Windows host with OneDrive Files On-Demand; `windows` / Cloud Filter APIs |
| **Option A (recommended)** | Expand detect honesty + errno hints on hydrate fail; separate evidence from DriveFS |
| **Option B** | Skip until OneDrive host available; keep residual Impossible + advisory |
| **Risks** | Reusing DriveFS receipts as WIN-003 Claimed |

### 7. FS-014 SELinux / AppArmor

| Field | Content |
|-------|---------|
| **Status** | **Partial** — host-policy Impossible for bypass; detect + errno honesty closest |
| **Blocker class** | Host LSM policy |
| **Deps** | Linux clean-room / Docker custom AppArmor profile; optional `selinux` crate |
| **Option A (recommended)** | Docker policy fixture + PermissionDenied hint naming LSM when detectable |
| **Option B** | Docs-only honesty; no fixture |
| **Risks** | Claiming “fixed SELinux” by detecting enforce=1 |

### 8. G19 / Pf5 **Claimed(partial)** (J-ALIAS-01 / J-REVERT-01)

| Field | Content |
|-------|---------|
| **Status** | **Partial** — M17 19/19 green; ≤0.5× misses process-tax PATH + short-wall quantum |
| **Blocker class** | Measurement physics / process tax (not functional FAIL) |
| **Deps** | Product PATH cut or amortize ADR path; sole G19 runner |
| **Option A (recommended)** | **Hold** — keep `aggressive_na`; do not thrash remasure after real gate FAIL |
| **Option B** | Single justified amortize wave (PATH argv / seed) → one sole remasure → accept PASS or re-NA |
| **Not options** | Lucky remasure loops; invent ≤0.5 PASS; concurrent G19 |
| **Risks** | Noise PASS; burning sole-runner capacity; rewriting historical 8/8 as tip without evidence |

### 9. G8 / G28 SKIP when secrets absent (honesty)

| Field | Content |
|-------|---------|
| **Status** | Expected **SKIP** without secrets; G28 live-private already Claimed with prior receipts |
| **Blocker class** | Same as §1 for *new* live latency samples |
| **Option A** | Keep SKIP ≠ PASS in CI/docs |
| **Option B** | Use `g8-local-forge` for wire-only substitute |
| **Risks** | CI green misread as live latency Claimed |

---

## Recommended sequence (ranked)

| Rank | Gap | Action | Owner / deps | Effort |
|-----:|-----|--------|--------------|--------|
| 1 | Live G8 latency | Secrets + remasure + evidence | Owner/operator secrets | Ops |
| 2 | Pf5 partial | Hold `aggressive_na`; optional single amortize later | Eng + sole G19 | Hold / S |
| 3 | WIN-003 / FS-007 advisory deepen | Detect/refuse slices; separate substrates | Eng + host | S–M |
| 4 | FS-014 | Docker LSM fixture + errno hints | Eng | S |
| 5 | L9 | Wait for owner SoT | Owner | — |
| 6 | Forge UX / crypto immunity | Keep Non-goal | — | — |

## Risks / refusals honored

- No invent PASS for secrets-absent live gates.
- No unauthorized L9 publish / `publish` flip.
- No G19 remasure stampede after ALIAS/REVERT miss.
- No new **M27** invent — tip currency lives in roadmap tip table + residual reopen queue.
- No Claimed full FS-007 / crypto immunity / forge Checks UX without primary SoT reopen.

## Cross-links

| Artifact | Role |
|----------|------|
| [[sot:root/program-roadmap]] v0.15.2 | Tip currency Done/Partial/Blocked/Non-goal table |
| [[sot:root/production-residual-register]] | Residual row dispositions |
| `docs/completion.md` / `docs/git-vs-arks.md` | Public PARTIAL honesty |
| [[research:root/20261110225000]] | FS_final currency verdict |
| [[research:root/20261110222000]] | FFI / external package paths |
| [[research:root/20261111012840]] | Post-U-ladder still-blocked + external-deps unblock options |
| [[evidence:root/20261110224814]] | Clean-room matrix re-stamp |
| [[evidence:root/20261111003444]] | U-ladder partial-gap close |
