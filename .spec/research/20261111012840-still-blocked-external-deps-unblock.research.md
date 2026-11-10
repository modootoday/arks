---
id: research:root/20261111012840
title: Still-blocked items — external-deps unblock options (post U-ladder)
status: active
created: 2026-11-10T16:28:40Z
updated: 2026-11-10T16:53:05Z
tags:
  [
    still-blocked,
    external-deps,
    u-ladder,
    g8-live,
    l9,
    pf5,
    j-alias-01,
    j-revert-01,
    sha1cd,
    immunity,
    fs-007,
    win-003,
    fs-014,
    forge-ux,
    operator-only,
    impossible,
  ]
verify: manual
rel:
  depends_on:
    - evidence:root/20261111003444
    - research:root/20261111001816
    - research:root/20261110222000
    - research:root/20261111003444
    - research:root/20261110211804
    - sot:root/program-roadmap
    - sot:root/production-residual-register
    - sot:root/pf5-journey-seed-amortization
    - adr:root/20261110162000
    - adr:root/20261110220000
---

# Still-blocked items — external-deps unblock options (post U-ladder)

## Question

After tip ≥ **`6ecd3e0`** and U-ladder [[evidence:root/20261111003444]] closed Partial gaps honestly, which **still-blocked / Impossible-to-research** items can be unblocked by **accepting external dependencies** (secrets, crates, OS APIs, `gh`, host substrates), and what are Plan A/B + verdicts — without inventing PASS, unauthorized L9 publish, or a G19 stampede?

## Method

1. Reconcile U-ladder dispositions + tip table ([[sot:root/program-roadmap]] v0.15.3) + residual §F + `docs/completion.md`.
2. Fold prior unblock research ([[research:root/20261111001816]], [[research:root/20261110222000]], [[research:root/20261111003444]], [[research:root/20261110211804]]).
3. Re-probe Environment `g8-live` secret **names only** (`total_count`).
4. For each item: root cause after U-ladder → external-deps options → tradeoffs → Plan A/B + acceptance → verdict.
5. Analysis only for Pf5 — propose next engineering step; **do not** run G19 remasure in this research task.

## Tip honesty baseline

| Item | Value |
|------|-------|
| Tip | ≥ `14082e6` (this research land) + unblock exec [[evidence:root/20261111015154]] |
| FS_final | **YES** |
| Operator one-word | **PARTIAL** |
| `g8-live` secret count | **0** (re-probed 2026-11-11 twice via `gh api …/environments/g8-live/secrets`) |
| Pf5 post-unblock | ALIAS **0.5546** `aggressive_na`; REVERT **0.5000 Claimed**; M17 **19/19**; claimed=18 ([[evidence:root/20261111015154]]); U1 archival ALIAS 0.7906 / REVERT 0.5417 |
| L9 | Owner-closed; reopen pack only (no in-repo reopen) |
| sha1cd | Detect+refuse **Claimed**; immunity **Impossible** |
| Host-env | Detect + opt-in refuse / LSM detect **Claimed** (+ Nextcloud marker advisory); full sync/hydration/bypass **Impossible** |
| Forge UX | Claimed-closest `arks forge` → host `gh` (+ recipe deepen); full product **Non-goal** |

Verdict vocabulary (this note):

| Verdict | Meaning |
|---------|---------|
| **Unblockable-with-ext-deps** | External package / OS API / Actions / host substrate can raise honesty class with evidence |
| **Operator-only** | No further in-tree research unlocks it; human secrets / owner SoT required |
| **Still Impossible** | No external dep removes the hard reason; keep Non-goal / docs honesty |

---

## 1. Live G8 latency (`g8-live` secrets=0)

### Root cause after U-ladder

U3 correctly refused invent PASS: Environment `g8-live` still has **`total_count=0`**. Harness + local-forge **wire** are Claimed; live latency samples remain SKIP. YAML/agents cannot mint Environment secret *values*.

### External-dependency options

| Dep | Role |
|-----|------|
| GitHub Environment secrets `ARKS_G8_TOKEN` + `ARKS_G8_REPO_URL` | Required for Actions `g8-forge-scale.yml` `dry_run=false` |
| Throwaway HTTPS peer + scoped PAT / fine-grained token | Measurement substrate |
| Optional: process-env same names + compose `g8` / `g8-live-scale` | Local remasure path |
| Optional: GitHub Actions sole-runner capacity | Avoid colliding G14–G28 / G19 |

### Tradeoffs

| Axis | Note |
|------|------|
| Security | PAT leak in logs; prefer throwaway repo + rotate after attach |
| Specfold honesty | SKIP / dry-run / local-forge `claimed-wire` ≠ live latency PASS |
| CI cost | One live dispatch + sole-runner Docker; do not stampede |
| Maintenance | Secrets rotation; Environment reviewers |

### Plan A / B

| Plan | Steps | Acceptance |
|------|-------|------------|
| **A (recommended)** | Owner fills `g8-live` secrets → confirm `total_count ≥ 2` → `gh workflow run g8-forge-scale.yml -f dry_run=false` → attach `.spec/evidence/<ts>-g8-live-remasure/` → residual R5 / tip table latency row Done only with receipts | Non-SKIP measured receipt; `github_live_pass` honest; no invented medians |
| **B** | Export same env names locally → compose `g8` + `g8-live-scale` → same evidence rules | Same as A with venue=`compose-live` |
| **C (steady)** | Leave Blocked; keep local-forge wire as substitute | Documented PARTIAL OK indefinitely |

### Verdict

**Operator-only** (temporarily). Ext-deps are ops secrets + Actions — not crates. Unblocks to live PASS **only after** human secret write + receipts.

---

## 2. L9 crates.io / docs.rs publish

### Root cause after U-ladder

U4 filed reopen pack [[research:root/20261111003444]] and correctly **did not publish**. `publish = false`; owner SoT reopen absent; agent authority = none.

### External-dependency options

| Dep | Role |
|-----|------|
| crates.io OIDC trusted publisher + Environment `crates` | Live publish path (L4 dry-run already Done) |
| Optional crates.io API token (operator vault; never commit) | Fallback if OIDC unbound |
| docs.rs | Follows publish automatically |
| `cargo publish --dry-run` / `scripts/release-channel-readiness.sh` | Structural only |

### Tradeoffs

| Axis | Note |
|------|------|
| License / public API | Premature freeze / yank churn |
| Security | No tokens in repo; Environment reviewers required |
| Specfold honesty | Dry-run ≠ crates.io PASS; agent MUST NOT invent L9 Done |
| Maintenance | Semver + CHANGELOG + yank policy |

### Plan A / B

| Plan | Steps | Acceptance |
|------|-------|------------|
| **A** | Owner SoT amend (public-surface + roadmap) → flip selected `publish` → tag `v*` → approve `crates` Environment → evidence with crates.io + docs.rs URLs → tip L9 **Done** | Live crate version URL + docs.rs build green |
| **B** | Keep closed; refresh `docs/release.md` readiness only | L9 stays Blocked; product DoD unaffected |
| **Not options** | Unauthorized `cargo publish`; treating dry-run as PASS |

### Verdict

**Operator-only** (owner-policy). Ops path known; not a development Open.

---

## 3. Pf5 J-ALIAS-01 / J-REVERT-01 ≤0.5× (Claimed partial)

### Root cause after U-ladder

U1 sole remasure ([[evidence:root/20261111003444]] `g19/`):

| Journey | git_ms | arks_ms | ratio | Class |
|---------|-------:|--------:|------:|-------|
| J-ALIAS-01 | 363 | 287 | **0.7906** | process-tax (alias PATH argv vs direct arks); residual 48→56 **hurt** — rolled back to 48 |
| J-REVERT-01 | 96 | 52 | **0.5417** | quantum-timer-floor; +12 timed amortize kept (was ~0.67) |

M17 ≤0.8× **19/19**. Umbrella stays **Claimed(partial)** with honest `aggressive_na`. Further seed inflation for ALIAS is exhausted / counterproductive; REVERT is near the 0.5 wall (`arks ≤ 48 ms` needed).

### External-dependency options (accepted if product wants ≤0.5)

| Dep / lever | Role |
|-------------|------|
| Stock git in-image (already oracle) | Keep comparative honesty |
| Alternate harness: higher `iterations`, warmer floor, or journey-local timer quantum SoT | Reduce REVERT flicker **without** inventing PASS |
| Product PATH cut: fewer argv layers / `exec`-style alias dispatch | Attack ALIAS process-tax (real engineering, not seed) |
| In-process multi-command amortize | Explicitly **Non-goal** today ([[sot:root/pf5-journey-seed-amortization]]); needs ADR |
| Sole G19 runner only | One remasure after a **justified** product change — never stampede |

### Tradeoffs

| Axis | Note |
|------|------|
| Specfold honesty | Lucky remasure loops = fraud; `aggressive_na` is the honest tip bar |
| CI cost | Each G19 is expensive; U1 already spent the controlled wave |
| Maintenance | Mega-seed OOM risk; PATH redesign may affect alias semantics |
| Measurement physics | ALIAS gap (~0.79) is structural process-tax; REVERT (~0.54) is short-wall quantum |

### Plan A / B (analysis only — **no G19 in this task**)

| Plan | Steps | Acceptance |
|------|-------|------------|
| **A (recommended hold)** | Keep Claimed(partial); forbid remasure thrash; document U1 ratios | Tip honesty unchanged; M17 stays green |
| **B (next engineering step — REVERT-first)** | Design a **single** product-justified change: (1) modest REVERT timed-amortize nudge **or** harness quantum floor SoT clarify for walls &lt;100 ms; (2) **or** ALIAS argv/PATH product cut (not seed bump). Land code → **one** sole G19 only → PASS ≤0.5 or re-`aggressive_na` | Receipt shows ≤0.5 for targeted journey(s) **or** honest NA retained; **no second remasure** after FAIL |
| **Not options** | Concurrent G19; seed 56+ for ALIAS without PATH redesign; invent ≤0.5 PASS |

**Next engineering step (proposed, not run):** Prefer REVERT micro-path (closer to bar) over ALIAS seed. For ALIAS, only a product PATH/`exec` redesign is plausible; seed amortization already failed U1.

### Verdict

**Unblockable-with-ext-deps / engineering** for REVERT (narrow); ALIAS likely needs **product PATH redesign** (not more crates). Steady-state **Claimed(partial)** remains valid. Not Operator-only; not Still Impossible as a functional FAIL.

---

## 4. SHA-1 collision **immunity** (detect Claimed)

### Root cause after U-ladder

U6: detect+refuse already Claimed (`sha1collisiondetection` / sha1cd path). Absolute **immunity** is a cryptographic math claim no library can make true.

### External-dependency options

| Dep | Role |
|-----|------|
| `sha1collisiondetection` (already integrated) | Detect+refuse Claimed bar |
| C `libsha1detectcoll` / git.git sha1dc | Alternate FFI; no immunity upgrade |
| SHAttered PDFs / known colliding pair fixtures | Regress detect path only |
| sha256 porcelain (already Claimed) | Avoid SHA-1 collision class for new repos |

### Tradeoffs

| Axis | Note |
|------|------|
| Specfold honesty | Marketing “immune” = invent Claimed |
| CPU | ~2× SHA-1 on detect path (accepted) |
| License | MIT Sequoia port — already OK |

### Plan A / B

| Plan | Steps | Acceptance |
|------|-------|------------|
| **A** | Keep wording: detect+refuse Claimed; immunity Non-goal / Impossible | Docs + residual OBJ-004 unchanged |
| **B** | Optional ingest-path coverage audit + SHAttered fixture regress | Detect still Claimed; immunity still Impossible |

### Verdict

**Still Impossible** for immunity. Detect path is Done — do not reopen as development Open.

---

## 5. Full cloud-sync / hydration / LSM bypass (FS-007 / WIN-003 / FS-014)

### Root cause after U-ladder

U2 deepened detect + opt-in refuse + LSM sysfs + errno honesty. Full multi-device sync safety, OneDrive hydration implementation, and LSM **bypass** remain host physics / policy — cannot be packaged away inside arks. DriveFS G: advisory ≠ WIN-003 CfAPI.

### External-dependency options (advisory deepen only)

| Gap | Ext dep | Raises to |
|-----|---------|-----------|
| FS-007 | DriveFS registry/`GoogleDriveFS` markers; Dropbox/Syncthing path markers; opt-in `--refuse-sync` (shipped); optional two-machine out-of-band protocol research | Stronger advisory; **not** full Claimed sync safety |
| WIN-003 | Real OneDrive Files On-Demand host; `windows` Cloud Filter / `RECALL_ON_DATA_ACCESS`; never reuse G: DriveFS receipts | Detect+errno already Claimed; hydration still Impossible |
| FS-014 | Docker custom AppArmor profile fixture; optional crates.io `selinux` / `libapparmor`; `getenforce`/`aa-status` | Detect+errno Claimed; bypass still Impossible |

### Tradeoffs

| Axis | Note |
|------|------|
| Specfold honesty | Thin-wrap detect ≠ “cloud-sync safe” / “SELinux fixed” |
| CI cost | OneDrive host not in Linux clean-room; Docker LSM fixture is cheap |
| Maintenance | OS API churn (CfAPI); SELinux policy fixtures brittle |
| Security | Refuse-sync reduces foot-guns; does not stop remote lock steal |

### Plan A / B

| Plan | Steps | Acceptance |
|------|-------|------------|
| **A** | Hold Claimed(detect+opt-in refuse / LSM detect); keep full sync/hydration/bypass Non-goal | Tip table honesty |
| **B** | Optional: OneDrive-host WIN-003 detect evidence; Docker AppArmor deny-lock fixture with `claimed=false` bypass | New advisory receipts only; residual Impossible rows unchanged for full claim |

### Verdict

**Still Impossible** for *full* cloud-sync safety / hydration implement / LSM bypass. **Unblockable-with-ext-deps** only for *further advisory substrates* (already largely Claimed).

---

## 6. Forge host UX (Non-goal) — can ext deps upgrade Claimed-closest?

### Root cause after U-ladder

U5 shipped `arks forge` thin wrap of host `gh` + `recipe` — **Claimed-closest**. Full Checks / PR / Issues / merge-queue / embedded OAuth remains product Non-goal (revision layer ≠ forge host).

### External-dependency options

| Dep | Upgrade shape |
|-----|---------------|
| Host `gh` on PATH / `$ARKS_GH` (shipped) | Forward PR/Checks/api |
| `octocrab` / `octorust` in-process REST | Richer helper **without** claiming forge product UX |
| Git Credential Manager / external `credential.helper` | OAuth already honored externally |
| Device-flow crates | Still AUTH SoT boundary for first-party browser OAuth |

### Tradeoffs

| Axis | Note |
|------|------|
| Specfold honesty | In-process GitHub client ≠ Claimed forge Checks product |
| License / legal | GitHub App registration if first-party OAuth |
| Maintenance | Track `gh` UX vs reimplement subset |
| Scope creep | Easy to market as forge parity — refuse |

### Plan A / B

| Plan | Steps | Acceptance |
|------|-------|------------|
| **A** | Keep thin-wrap; docs “use `gh`”; no product Claimed | Claimed-closest steady |
| **B** | Optional: more `arks forge recipe` coverage or thin `octocrab` helper behind explicit “not forge UX” notes | Still Partial(closest); full UX stays Non-goal until primary SoT reopen |

### Verdict

**Still Impossible** as *in-arks forge host product Claimed* without SoT reopen. **Unblockable-with-ext-deps** only for *Claimed-closest deepen* (optional; low priority).

---

## Ranked unblock options (operator / eng)

| Rank | Item | Verdict | Fastest honest next |
|-----:|------|---------|---------------------|
| 1 | Live G8 latency | **Operator-only** | Fill `g8-live` secrets → live dispatch → evidence |
| 2 | L9 publish | **Operator-only** | Owner SoT reopen pack only — no agent publish |
| 3 | Pf5 ALIAS/REVERT | **Unblockable-with-eng** (REVERT nearer) | Hold partial **or** one justified product/harness change → **one** sole G19 |
| 4 | Host-env advisory deepen | **Unblockable-with-ext-deps** (advisory only) | Optional OneDrive / Docker LSM fixtures; full claim Impossible |
| 5 | Forge Claimed-closest deepen | **Optional / Non-goal product** | More `gh` recipes or thin API helper; do not Claim full UX |
| 6 | SHA-1 immunity | **Still Impossible** | Docs only; detect Claimed |

## Refusals honored (this research)

- No invent PASS for secrets-absent live G8.
- No unauthorized L9 / `publish` flip.
- No G19 remasure / stampede in this research-only draft (Pf5 analysis only).
- No Claimed full FS-007 / WIN-003 hydration / FS-014 bypass / crypto immunity / forge Checks product.

## Execution (post-research land tip ≥ `14082e6`)

| Rank | Action taken | Result |
|-----:|--------------|--------|
| 3 Pf5 | REVERT amortize 12→20 → **one** sole G19 `sole-rm-pf5-ub` | REVERT **0.5000 Claimed**; ALIAS **0.5546** `aggressive_na`; claimed=18; M17 19/19; **no second remasure**; no ALIAS PATH redesign |
| 1 G8 | Re-probe `g8-live` secrets | **total_count=0** → operator steps refreshed in `docs/forge-scale.md` only; **no** invent live PASS |
| 2 L9 | Scan for owner SoT reopen | **Absent** → leave Blocked; reopen pack unchanged |
| 5 Forge | Deepen `arks forge recipe` (`gh` Checks/workflow/env probes) | Claimed-closest only; full UX still Non-goal |
| 4 FS | Cheap Nextcloud/ownCloud journal marker detect | Advisory deepen only; full sync still Impossible |
| 6 Immunity | Docs honesty only | Detect Claimed; immunity Impossible — no fake immune |

Evidence: [[evidence:root/20261111015154]].

## Cross-links

| Artifact | Role |
|----------|------|
| [[evidence:root/20261111015154]] | Unblock sole G19 (REVERT Claimed / ALIAS NA) |
| [[evidence:root/20261111003444]] | U-ladder pack (Pf5 ratios, host-env, local-forge, L9 reopen cite) |
| [[research:root/20261111001816]] | Prior per-gap unblock roadmap |
| [[research:root/20261110222000]] | FFI / external package deep dive |
| [[research:root/20261111003444]] | L9 owner reopen checklist |
| [[sot:root/program-roadmap]] v0.15.4 | Tip Done/Partial/Blocked/Non-goal table |
| `docs/completion.md` | Public PARTIAL honesty |
