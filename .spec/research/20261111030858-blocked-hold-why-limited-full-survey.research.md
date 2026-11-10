---
id: research:root/20261111030858
title: Blocked / Hold / aggressive_na / Impossible / Non-goal — why implementation is limited (full survey)
status: active
created: 2026-11-10T18:08:58Z
updated: 2026-11-10T18:08:58Z
tags:
  [
    blocked,
    hold,
    aggressive-na,
    impossible,
    non-goal,
    why-limited,
    g008-live-forge-smoke-live,
    l9,
    pf5,
    j-alias-01,
    sha1-immunity,
    host-env,
    celir-objectstore,
    public-surface,
    forge-ux,
    lfs-tus,
    sparse-oid,
    mixed-hash,
    force-with-lease,
    git-pin,
    g-ladder,
  ]
verify: manual
rel:
  depends_on:
    - sot:root/program-roadmap
    - sot:root/production-residual-register
    - sot:root/public-surface
    - sot:root/product-intent
    - sot:root/pf5-journey-seed-amortization
    - sot:root/storage-celir-boundary
    - sot:root/clean-room-git-pin-advance
    - sot:root/arks-remote
    - sot:root/interop-test-matrix
    - plan:root/20261111024440
    - plan:root/20261111024441
    - research:root/20261111022345
    - research:root/20261111012840
    - research:root/20261111003444
    - research:root/20261111001816
    - research:root/20261110212414
    - evidence:root/20261111025000
    - evidence:root/20261111025200
    - evidence:root/20261111025300
    - evidence:root/20261111015154
    - adr:root/20261110162000
    - adr:root/20261110214800
    - adr:root/20261110220000
    - adr:root/20261110052030
---

# Blocked / Hold / aggressive_na / Impossible / Non-goal — why implementation is limited (full survey)

## Question

At tip ≥ **`b7720d9`**, with G-ladder **G_final Done** and operator honesty **PARTIAL**, inventory every residual still labeled **Blocked / Hold / aggressive_na / Impossible / Non-goal / Partial(non-claim)** and explain **why implementation is limited** for each — without inventing PASS, thrashing G19, or publishing L9.

## Method

1. Reconcile tip currency ([[sot:root/program-roadmap]] v0.15.5), residual §F ([[sot:root/production-residual-register]] v0.9.9), public Pages (`docs/completion.md`, `docs/limits.md`, `docs/git-vs-arks.md`, `docs/forge-scale.md`).
2. Fold G-ladder plans/evidence ([[plan:root/20261111024440]], [[plan:root/20261111024441]], [[evidence:root/20261111025000]], [[evidence:root/20261111025200]], [[evidence:root/20261111025300]]), classification [[research:root/20261111022345]], external-deps [[research:root/20261111012840]], L9 reopen pack [[research:root/20261111003444]], public-surface [[sot:root/public-surface]].
3. Re-probe Environment `g008-live-forge-smoke-live` secrets **names only** (`total_count`).
4. For each scoped item: tip status → why limited (cite SoT) → unblock precondition → class.
5. Research only — **no** G19 remasure, **no** invent PASS, **no** L9 publish / `publish` flip.

## Tip honesty baseline (2026-11-11)

| Item | Value |
|------|-------|
| Tip | **`b7720d9`** (*Fix clippy collapsible_match in claimed filter object:type gate.*) |
| FS_final | **YES** |
| Operator one-word | **PARTIAL** |
| Pf5 | claimed=18 + J-ALIAS-01 `aggressive_na` **0.5489**; REVERT **0.4943 Claimed**; M17 **19/19** ([[evidence:root/20261111025200]]) |
| `g008-live-forge-smoke-live` secrets | **`total_count=0`** (re-probe this survey; matches [[evidence:root/20261111025000]]) |
| L9 | Owner-closed; all crates `publish = false`; reopen pack only |
| G3–G6 | Server `bundle-uri` / LFS locks / forge recipe / `object:type`+`combine:` **Claimed** ([[evidence:root/20261111025300]]); `sparse:oid` Hold; tus/S3 handoff |

### Class vocabulary (this survey)

| Class | Meaning |
|-------|---------|
| **Operator** | Human fills secrets / runs live dispatch / attaches receipts; eng code cannot mint PASS |
| **Owner** | Requires owner SoT amend / policy reopen; agents lack authority |
| **Eng-hard** | Implementable in-tree (or with deps) but non-trivial; honesty may stay Partial until wire+evidence |
| **Physics/Crypto** | Host physics, crypto math, or measurement floor — no honest Claimed upgrade to the absolute bar |
| **Intentional Non-goal** | Product-intent / Specfold refuse / public-surface policy forbids the claim |

---

## Per-item survey

### 1. Live G8 latency (secrets=0)

| Field | Content |
|-------|---------|
| **Tip status** | **Blocked** (G1 Done-as-honesty). Harness **Claimed** (default-on when secrets); local-forge `claimed-wire` ≠ GitHub live PASS. |
| **Why limited** | Environment `g008-live-forge-smoke-live` has **zero** secrets (`ARKS_G8_TOKEN` / `ARKS_G8_REPO_URL` absent). YAML and agents cannot invent Environment secret *values*. Latency medians without live samples would be invented PASS — forbidden ([[adr:root/20261110220000]], [[adr:root/20261110221000]], `docs/forge-scale.md`, residual R5 / **FW-G8-DEFAULT** / **FW-GH-LIVE-SCALE**). |
| **Unblock** | Owner/operator: mint throwaway peer + scoped PAT → set Environment secrets (`total_count ≥ 2`) → `gh workflow run g008-live-forge-smoke-forge-scale.yml -f dry_run=false` (or compose `g008-live-forge-smoke` + `g008-live-scale`) → attach `.spec/evidence/` with real receipts. Force-off remains `ARKS_G8_LIVE=0`. |
| **Class** | **Operator** |

SoT: [[sot:root/program-roadmap]] tip row; [[evidence:root/20261111025000]]; [[sot:root/interop-test-matrix]]; `docs/forge-scale.md`.

---

### 2. L9 crates.io / docs.rs

| Field | Content |
|-------|---------|
| **Tip status** | **Blocked** (G2 Done-as-honesty). Release-channel **dry-run / OIDC wiring Done (L4 ops)** ≠ live registry PASS. |
| **Why limited** | Owner policy closed publish timing ([[sot:root/public-surface]] crates.io row; ladder L9). All workspace crates keep `publish = false`. No in-repo SoT reopen flipping L9 to Open/Done. Agent authority for unauthorized `cargo publish` = **none**. |
| **Unblock** | Owner SoT amend (public-surface + roadmap L9) → flip selected `publish` → trusted publisher / Environment `crates` → tag `v*` → evidence with crates.io + docs.rs URLs. Checklist: [[research:root/20261111003444]]. |
| **Class** | **Owner** |

---

### 3. Pf5 J-ALIAS-01 `aggressive_na` (~0.5489)

| Field | Content |
|-------|---------|
| **Tip status** | **aggressive_na** / umbrella **Claimed(partial)**. G7 sole G19: ratio **0.5489** (git=368 / arks=202 ms); was 0.5546 before PATH/`exec` absolute-shim; still above ≤0.5×. REVERT **0.4943 Claimed**. M17 ≤0.8× **19/19**. |
| **Why limited** | Alias journey is **process-tax**: argv/PATH shim layer vs direct `arks`. Seed amortization exhausted / hurt (U1 residual 48→56 rolled back). G7 absolute-path shim cut improved slightly but did **not** clear 0.5. Further remasure without a new product cut = thrash/fraud risk ([[sot:root/pf5-journey-seed-amortization]] Non-goals; [[adr:root/20261110162000]]). In-process multi-command amortize remains Non-goal without ADR. |
| **Unblock** | Only a **product-justified** further PATH/`exec` redesign (or ADR-approved amortize) → **exactly one** sole Docker G19 → ≤0.5 Claimed **or** keep `aggressive_na`. **No stampede.** |
| **Class** | **Eng-hard** (hold tip honesty; optional future product cut) |

SoT: [[evidence:root/20261111025200]], residual **FW-PF5-J-ALIAS** / **FW-PF5-NA**, [[sot:root/pf5-journey-seed-amortization]].

---

### 4. SHA-1 collision immunity

| Field | Content |
|-------|---------|
| **Tip status** | Detect+refuse **Claimed** (`sha1collisiondetection` / OBJ-004); absolute **immunity** = **Impossible / Non-goal / Hold forever**. |
| **Why limited** | Cryptographic **math**: no library can truthfully claim immunity to theoretical collisions. Detect+refuse is the honest product bar. Marketing “immune” would invent Claimed ([[adr:root/20261110223000]], residual **FW-CAT-OBJ-004**, `docs/limits.md`). |
| **Unblock** | **Nothing** for absolute immunity. Optional: more detect fixtures (SHAttered) — still does not upgrade immunity. Prefer sha256 porcelain (already Claimed) for new repos. |
| **Class** | **Physics/Crypto** |

---

### 5. Full cloud-sync / OneDrive hydration / LSM bypass

| Field | Content |
|-------|---------|
| **Tip status** | Detect + opt-in refuse / CfAPI detect / LSM detect+errno **Claimed(advisory)**; full multi-device sync safety / hydration implement / LSM **bypass** = **Impossible / Non-goal / Hold forever** (FS-007 / WIN-003 / FS-014). |
| **Why limited** | **Host-env physics + policy**: multi-device sync races on `.git`, OneDrive Files On-Demand hydration, and SELinux/AppArmor enforcement live outside the revision engine. Crates cannot warranty “cloud-sync safe” or “SELinux fixed.” DriveFS G: ≠ OneDrive placeholders ([[research:root/20261110222000]]). |
| **Unblock** | Full Claimed sync/hydration/bypass: **none honest**. Advisory deepen (extra markers / Docker LSM fixtures) only — residual Impossible rows stay. |
| **Class** | **Physics/Crypto** (host physics) + **Intentional Non-goal** for warranty language |

Residual: **FW-CAT-FS-007**, **FW-CAT-WIN-003**, **FW-CAT-FS-014**.

---

### 6. CELIR ObjectStore refuse-closed

| Field | Content |
|-------|---------|
| **Tip status** | **Refuse-closed (terminal)** — **FW-CELIR-OS**. Index dual-path M24 / `celir-index` **Claimed** (opt-in); ObjectStore path forbidden. |
| **Why limited** | **Product-intent + CELIR VCS Refuse**: filesystem `.git` is sole product object store ([[sot:root/product-intent]], [[sot:root/storage-celir-boundary]]). CELIR refuses put/get/exists Git object API. Reverse ObjectStore ADR is an explicit **forbidden product path**. |
| **Unblock** | Would require primary product-intent reopen + reverse ADR — treated as **not scheduled** / Hold forever. Do not reintroduce via crates. |
| **Class** | **Intentional Non-goal** |

---

### 7. “Faster than Git” marketing Non-goal

| Field | Content |
|-------|---------|
| **Tip status** | **Hold forever / Closed (policy)** — public-surface forbids blanket slogan. |
| **Why limited** | Comparative ratios are **evidence-bound** (M17/Pf5 G19). Blanket marketing ≠ feature gap and would overclaim vs journey-local receipts ([[sot:root/public-surface]] §22; `docs/limits.md` Performance wording; residual tip Hold-forever bundle). |
| **Unblock** | **None** for blanket claim. Journey-local “≤0.8× / ≤0.5× with receipt” language remains the only honest form. |
| **Class** | **Intentional Non-goal** |

---

### 8. Multi-writer warranty non-claim

| Field | Content |
|-------|---------|
| **Tip status** | Lease L3 / shared-index / catalog **Claimed** with receipts; **not** a warranty that every multi-writer race is solved (`docs/limits.md` Concurrency honesty). Catalog Deferred=0 ≠ “all races fixed.” |
| **Why limited** | Absolute warranty language is dishonest Specfold marketing. Claimed sets are scenario-bounded. Further races = new catalog IDs only, not a universal guarantee ([[sot:root/multi-agent-git-concurrency-scenarios]], M21 exit notes). |
| **Unblock** | Absolute warranty: **none**. Optional new scenario IDs via SoT amend + gates. |
| **Class** | **Intentional Non-goal** |

---

### 9. GUI / scalar product embed

| Field | Content |
|-------|---------|
| **Tip status** | GUI **Claimed (FS7 handoff)** refuse+reason; `scalar` **Claimed (FS6 refuse)** — not core git porcelain. Embed = **Non-goal / Hold forever**. |
| **Why limited** | Product-intent Non-goal: embedding Tcl/Tk / Scalar VFS product inside CLI expands scope beyond revision layer ([[sot:root/product-intent]] Non-goals §7; residual **FW-GUI** / **FW-SCALAR**; FS ladder FS6/FS7). |
| **Unblock** | Handoff already Done. Product embed requires primary SoT reopen + new GUI/Scalar product — not scheduled. |
| **Class** | **Intentional Non-goal** |

---

### 10. Embedded forge OAuth browser UX

| Field | Content |
|-------|---------|
| **Tip status** | External `credential.helper` OAuth **Claimed** (honor); embedded browser OAuth **Refuse / Impossible** under AUTH. |
| **Why limited** | First-party browser OAuth = forge auth **product** + App registration / legal surface. G33 CRED claims helpers/TTY/OS wallets — MUST NOT market forge OAuth under CRED ([[sot:root/public-surface]] §32; residual **FW-AUTH-PROXY** Refuse forge OAuth; **FW-CRED-ASKPASS** Claimed without OAuth). |
| **Unblock** | AUTH SoT reopen + App registration + browser/device-flow UX **or** keep documenting GCM/helper OAuth. Preferred: external helper forever. |
| **Class** | **Intentional Non-goal** (Owner if AUTH reopen) |

---

### 11. Full forge host UX (beyond `arks forge` → `gh`)

| Field | Content |
|-------|---------|
| **Tip status** | **Claimed-closest** thin-wrap + recipe deepen (**G5 Done**); full Checks / PR / Issues / merge-queue **product** UX = **Impossible / Non-goal** under FS DoD (**FW-FORGE-B**). |
| **Why limited** | Forge host APIs are **not git CLI features** ([[sot:root/product-intent]], [[sot:root/remote-host-compatibility-surface]] scope B). `gh` already owns that surface; embedding equals a second forge product. |
| **Unblock** | Full Claimed forge product: primary SoT reopen (Intentional Non-goal today). Optional recipe/`octocrab` deepen stays Claimed-closest only. |
| **Class** | **Intentional Non-goal** |

---

### 12. LFS tus / multi-tenant S3 (handoff)

| Field | Content |
|-------|---------|
| **Tip status** | Batch/basic network + `serve-http` `info/lfs` + **locks** subset **Claimed (G4)**; tus / custom transfers / multi-tenant S3 = **Partial / handoff** (residual **FW-LFS** reopen note). |
| **Why limited** | tus and S3 custom transfer agents are **external transfer products** (typically `git-lfs` / AWS agents / tusd). Core revision engine already covers batch/basic + locks; full S3 multi-tenant product is ops+agent scope ([[adr:root/20261110214800]], [[evidence:root/20261111025300]] Non-claims). |
| **Unblock** | Handoff deepen to `git-lfs` for exotic argv (**preferred**) **or** implement tus adapter + S3 agent with SoT amend + evidence (Eng-hard, L effort) — still may stay Partial vs full git-lfs parity. |
| **Class** | **Eng-hard** (optional) / **Intentional Non-goal** as default production gate |

---

### 13. `sparse:oid` / remaining exotic filters Hold

| Field | Content |
|-------|---------|
| **Tip status** | Claimed filters: `blob:none`, `tree:0`, `tree:<n>`, `blob:limit=…`, `object:type=…`, `combine:` of Claimed children (**G6**). **`sparse:oid=…` Hold / refuse fail-closed**; other unclaimed specs N/A ([[sot:root/arks-remote]] rule 15). |
| **Why limited** | `sparse:oid` needs sparse-OID tree walking / blob omission tied to a sparse-checkout OID — hardest remaining filter family (classification A1). Not blocked on crates (`gix` forbidden); pure protocol Eng-hard. FS_final already honest without casual N/A. |
| **Unblock** | SoT amend + pack-objects/promisor wire + G39-class evidence per family. Schedule only if product needs sparse-OID clones. |
| **Class** | **Eng-hard** (Hold until product need) |

---

### 14. Mixed sha1 ↔ sha256 remotes

| Field | Content |
|-------|---------|
| **Tip status** | sha256 porcelain **Claimed**; mixed-hash remotes **non-claim / Hold** ([[adr:root/20261110214800]] Non-claim; residual OBJ-004 reopen note). |
| **Why limited** | Cross-hash-function fetch/push needs object rewriting or dual-OID maps (`compatObjectFormat` / transition story). Hasher crates already present — **not** a dep gap. Stock Git treats this as advanced/compat edge; full product Claimed would be a major research ladder. |
| **Unblock** | Honest refuse on mismatched `extensions.objectFormat` (Claimable today). Full mixed remotes: primary SoT + long Eng-hard ladder — **not scheduled**. |
| **Class** | **Eng-hard** near-**Hold** / treat as **Intentional Non-goal** until SoT reopen |

---

### 15. Dumb HTTP `force-with-lease` (still non-claim)

| Field | Content |
|-------|---------|
| **Tip status** | Smart-pack `push --force-with-lease` + `--push-option` **Claimed (O4)**; **lease on dumb HTTP is not** (`docs/limits.md`, `docs/git-vs-arks.md`). |
| **Why limited** | Dumb WebDAV push has **no pack negotiation channel** for lease tokens. Claiming lease without a product-defined CAS over LOCK/PUT/MOVE would invent protocol. Apache G41 covers write subset, not lease semantics ([[research:root/20261111022345]] A2). |
| **Unblock** | Design CAS-over-DAV (or documented smart-only forever) + SoT amend + G41-adjacent matrix. Low ROI vs smart-pack. |
| **Class** | **Eng-hard** (optional) / documented asymmetry OK indefinitely |

---

### 16. Clean-room Git pin process-closed

| Field | Content |
|-------|---------|
| **Tip status** | **Closed (process)** — pin **Git 2.39.5** (`verify/images.lock.json`); residual **FW-PIN** / R6. |
| **Why limited** | Pin-advance is **hygiene process**, not a feature Open. Silent bump would carry stale “verified” claims. Current pin remains authoritative until deliberate gap audit + rebuild + G14-class re-prove ([[sot:root/clean-room-git-pin-advance]]). |
| **Unblock** | Follow pin-advance SoT when forced (security/distro/man-page retarget): audit → lock update → matrix → clean-room re-prove → evidence → public honesty. |
| **Class** | **Owner** (deliberate decision) + **Eng-hard** (matrix cost) — **not** a Blocked feature gap |

---

## Other tip residual Blocked / Hold / NA rows (catch-all)

| Item | Tip status | Why limited | Unblock | Class |
|------|------------|-------------|---------|-------|
| CDN / scale SLOs for `bundle-uri` | Non-claim (G3 Claimed advertise ≠ CDN SLO) | Throughput SLOs need live/ops measurement; same honesty class as G8 | Secrets+receipt / separate gate | **Operator** |
| Interactive TUI full terminal UX beyond FS1/FS3 | Honest defer / Claimed subset | Full TUI product ≠ symmetry DoD remainder | SoT reopen if product wants richer TUI | **Intentional Non-goal** (subset Claimed) |
| Catalog N/A ×9 (concurrency) | **N/A** terminal | Journey-class exclusions — not unfinished Opens | Do not re-litigate as Opens | **Intentional Non-goal** |
| Host CI vs Docker Verified | Advisory ≠ Verified | Product claims require clean-room gates | Keep Docker gates normative | **Intentional Non-goal** (host-only Done forbidden) |
| In-process multi-command Pf5 amortize daemon | Non-goal | Would change alias/process model | ADR + product SoT | **Intentional Non-goal** |
| Forge live throughput / Checks / merge-queue SLO | Non-claim | Host forge capacity ≠ arks revision product | Never invent; harness-only | **Intentional Non-goal** + **Operator** if measuring |

Bare development Opens remain **0**. §F rows are terminal Claimed/Refuse/N/A/Closed — this survey explains **non-FS / Hold / Blocked honesty**, not unfinished M22 work.

---

## Summary matrix

| # | Item | Tip status | Why limited (one line) | Class |
|---|------|------------|------------------------|-------|
| 1 | Live G8 latency | **Blocked** | `g008-live-forge-smoke-live` secrets=0; cannot invent medians | **Operator** |
| 2 | L9 crates.io/docs.rs | **Blocked** | Owner policy; `publish=false`; no agent publish | **Owner** |
| 3 | J-ALIAS-01 | **aggressive_na 0.5489** | Process-tax; PATH cut insufficient; no thrash | **Eng-hard** |
| 4 | SHA-1 immunity | **Impossible** | Crypto math; detect+refuse is the bar | **Physics/Crypto** |
| 5 | Cloud-sync / hydration / LSM bypass | **Impossible** | Host physics; advisory only Claimed | **Physics/Crypto** |
| 6 | CELIR ObjectStore | **Refuse-closed** | Product-intent + CELIR VCS Refuse | **Intentional Non-goal** |
| 7 | “Faster than Git” | **Non-goal** | Public-surface forbids blanket slogan | **Intentional Non-goal** |
| 8 | Multi-writer warranty | **Non-claim** | Claimed sets ≠ universal warranty | **Intentional Non-goal** |
| 9 | GUI / scalar embed | **Non-goal** | Handoff/refuse Claimed; embed out of scope | **Intentional Non-goal** |
| 10 | Embedded OAuth browser | **Refuse/Impossible** | AUTH/forge product; helper honor Claimed | **Intentional Non-goal** |
| 11 | Full forge host UX | **Non-goal** | Not a git CLI feature; `gh` thin-wrap closest | **Intentional Non-goal** |
| 12 | LFS tus / multi-tenant S3 | **Partial/handoff** | External transfer agents; locks Claimed | **Eng-hard** / Non-goal as default gate |
| 13 | `sparse:oid` | **Hold** | Hardest remaining filter; refuse fail-closed | **Eng-hard** |
| 14 | Mixed sha1↔sha256 remotes | **Non-claim/Hold** | Dual-OID rewrite research; not a hasher gap | **Eng-hard** / Hold |
| 15 | Dumb HTTP force-with-lease | **Non-claim** | No lease channel on WebDAV dumb push | **Eng-hard** (optional) |
| 16 | Clean-room Git pin | **Process-closed** | Hygiene SoT; pin 2.39.5 until deliberate bump | **Owner** + Eng process |

### What is *not* limited (G-ladder Claimed / Done — for contrast)

| Item | Status |
|------|--------|
| FS_final git feature symmetry DoD | **YES** |
| Server `bundle-uri` advertise (G3) | **Claimed** |
| LFS locks subset (G4) | **Claimed** |
| Forge recipe deepen (G5) | **Claimed-closest** |
| Filters `object:type` / `combine:` (G6) | **Claimed** |
| Pf5 REVERT ≤0.5× | **Claimed** (0.4943) |
| sha1cd detect+refuse | **Claimed** |
| Host-env detect + opt-in refuse / LSM detect | **Claimed(advisory)** |

---

## Ranked “what would change PARTIAL?”

| Rank | Action | Moves tip how | Who |
|-----:|--------|---------------|-----|
| 1 | Fill `g008-live-forge-smoke-live` secrets → live remasure + evidence | Blocked → live latency Done (ops) | **Operator** |
| 2 | Owner SoT reopen L9 → publish | Blocked → L9 Done | **Owner** |
| 3 | Further PATH/`exec` ADR → one G19 | Maybe clear ALIAS `aggressive_na` | **Eng-hard** |
| 4 | `sparse:oid` / dumb lease / tus (optional) | Feature Partial → Claimed slice | **Eng-hard** |
| — | Immunity / ObjectStore / blanket faster / multi-writer warranty / GUI·scalar embed / full forge UX / full sync | **Never** fake Claimed | Hold forever |

---

## Refusals honored

- No invent PASS for secrets-absent live G8 (re-probe still **0**).
- No unauthorized L9 / `publish` flip / `cargo publish`.
- No G19 remasure or stampede in this research task.
- No invented Claimed for immunity, ObjectStore, full sync/hydration/LSM bypass, forge Checks product, or blanket “faster than Git.”

## Cross-links

| Artifact | Role |
|----------|------|
| [[sot:root/program-roadmap]] v0.15.5 | Tip Done/Partial/Blocked/Non-goal table |
| [[sot:root/production-residual-register]] v0.9.9 | §F terminal + reopen paths |
| [[research:root/20261111022345]] | Self/dep/FFI gap classification |
| [[research:root/20261111012840]] | External-deps Plan A/B |
| [[research:root/20261111003444]] | L9 owner reopen checklist |
| [[evidence:root/20261111025000]]…[[evidence:root/20261111025300]] | G-ladder receipts |
| `docs/completion.md` / `docs/limits.md` / `docs/git-vs-arks.md` / `docs/forge-scale.md` | Public PARTIAL honesty |
