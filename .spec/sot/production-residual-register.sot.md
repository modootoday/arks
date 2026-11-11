---
id: sot:root/production-residual-register
title: Production residual register
status: active
created: 2026-11-08T14:27:58Z
updated: 2026-11-11T03:35:56Z
tags: [residual, production, m9, m10, m11, m12, m13, m14, m15, m16, m17, m18, m19, m20, m21, m22, m23, m24, m25, m26, g028-live-private-arks, g029-server-dumb-http, g030-server-ssh-command, g031-server-protocol-v2, g032-git-daemon-wire, g033-cred-askpass, g034-shared-index-contention, g035-shallow-depth, g036-http-basic-auth, g037-gpgsig-ssh, honesty, refuse-close, worktree, submodule, concurrency, full-symmetric, complete-symmetry, pf4, cx, dz, deferred-zero, residual-terminal, l3, credential, s0-s-final, claimed-residual, implement-wave, in-scope-leftovers]
verify: manual
rel:
  depends_on:
    - sot:root/program-roadmap
    - sot:root/cli-surface
    - sot:root/cli-flag-parity-m9
    - sot:root/cli-flag-parity-full
    - sot:root/cli-flag-parity-complete
    - plan:root/20261109035335
    - plan:root/20261109043910
    - plan:root/20261109043916
    - sot:root/public-surface
    - sot:root/arks-remaining-implement-wave
    - sot:root/arks-remaining-incompletes-wave
    - sot:root/arks-honesty-leftovers-wave
    - sot:root/arks-in-scope-leftovers-wave
    - research:root/20261111123556
    - research:root/20261111114312
    - research:root/20261111105924
    - research:root/20261111090827
    - research:root/20261111084400
    - sot:root/lease-concurrency-intent
    - sot:root/arks-lease
    - sot:root/arks-server
    - sot:root/clean-room-git-pin-advance
    - sot:root/clean-room-journey-compare
    - sot:root/product-verification-clean-room
    - sot:root/interop-test-matrix
    - plan:root/20261108183812
    - plan:root/20261108183813
    - plan:root/20261108105446
    - plan:root/20261109022956
    - plan:root/20261109024634
    - plan:root/20261109024635
    - plan:root/20261109024636
    - plan:root/20261109125438
    - plan:root/20261110010000
    - plan:root/20261110140000
    - plan:root/20261110150000
    - plan:root/20261109194208
    - sot:root/multi-agent-git-concurrency-scenarios
    - sot:root/crate-architecture
    - evidence:root/20261108150000
    - evidence:root/20261108231249
    - evidence:root/20261108231300
    - adr:root/20261108232800
    - adr:root/20261109194200
    - adr:root/20261109122126
    - adr:root/20261110151000
    - adr:root/20261110160000
    - adr:root/20261110161000
    - adr:root/20261110162000
    - evidence:root/20261110170000
    - adr:root/20261109220230
    - plan:root/20261109220230
    - evidence:root/20261109221500
    - evidence:root/20261109222000
    - evidence:root/20261109222500
    - evidence:root/20261109223000
    - evidence:root/20261110200000
  evidence:
    - evidence:root/20261108233129
    - evidence:root/20261109114207
    - evidence:root/20261109203000
    - evidence:root/20261109061600
    - evidence:root/20261109063000
    - evidence:root/20261109090000
    - evidence:root/20261109122512
    - evidence:root/20261109130000
    - evidence:root/20261109141500
    - evidence:root/20261109143000
    - evidence:root/20261110010000
    - evidence:root/20261110152000
    - evidence:root/20261110153000
    - evidence:root/20261110154000
    - evidence:root/20261109194858
    - evidence:root/20261109195122
    - evidence:root/20261109203236
    - evidence:root/20261109204428
    - evidence:root/20261109212500
    - evidence:root/20261109213400
    - evidence:root/20261110142500
    - evidence:root/20261110131600
---

# Production residual register

Version **0.9.12**. Canonical register of **post-M9/M10/M11 residual opens** closed by implement, refuse-close, N/A exclusion, or explicit disposition — so public and Specfold claims stay honest without inventing a silent fourth bar. **M23** L3 claimed shape is **Claimed** ([[evidence:root/20261109194858]]). **M25** server transports **G29–G32 Claimed**. **M26** shared-index N-way is **Claimed + G34 Verified** ([[adr:root/20261110074000]], [[evidence:root/20261110074200]]) — additive to L3; historical M22 refuse ADR ([[adr:root/20261110160000]]) is superseded for this bar only. **M24** is **CELIR index dual-path / migration** ([[plan:root/20261109203430]]) — Claimed (host advisory) + **G22–G25 Verified**; ObjectStore Refuse retained. **G26** local PR-sim Claimed; **G27** arks smart-HTTP PR-contention Claimed; **G28** live-private journeys Claimed ([[evidence:root/20261109212500]]) + offline clone PASS ([[evidence:root/20261109213400]]); checklist N/A=0; G28 land tip ~`902d728`. W1–W5 residual exits Claimed where evidenced: **CRED G33**, hooks honor list, **GPGSIG** SSH create/verify (**G37**; OpenPGP N/A), **G9 `--depth` / G35** (filter/promisor N/A), **AUTH Basic G36** (OAuth Refuse), **PF5 Claimed 8/8** (primary [[evidence:root/20261110142500]]), **FW-G8-DEFAULT** / **FW-GH-LIVE-SCALE** harness. Live forge **scale** remains harness + secrets-gated ([[adr:root/20261110161000]] / [[adr:root/20261110221000]]). Bare development Opens remain **0**; §F future-work inventory is **38 terminal rows** (P0–P6) — no bare Deferred.

**Bars:** M9 = historical CLI-parity inventory close; M15 = historical full CLI inventory residual; **M16 = active complete flag-symmetry alias bar** (implement \| N/A only); **M17 = full G19 = all G16 journeys**; M10 = lease capability; M11 = production-server. Tracks **M12/M13** (G17/G18), **M14** (frozen G19 allow-list), **M15** (G20), **M16** (G21), **M17** (expanded G19), **M21** (catalog Deferred=0), **M22** (residual terminal close) are in [[sot:root/program-roadmap]] and do **not** redefine M9/M10/M11 historical exits.

**Development residual exit (v0.6.0):** after M0–M17 claimed exits, this register MUST show **no unfinished development Opens**.

**M22 residual terminal exit (v0.7.0):** [[plan:root/20261110150000]]. **Closed-deferred is forbidden as a lasting work-item disposition.** Every prior Closed-deferred / journey-class “keep ≤0.8×” leftover MUST become **Closed (implement+verify)**, **Refuse-closed (ADR+SoT)**, or **N/A (primary-doc/product-intent cite)**. Silent keep-at-0.8 for ≤0.5× leftovers is forbidden.

Carrying exits (do not reopen as “not done”):

| Bar | Evidence | Gate |
|-----|----------|------|
| M9 CLI-parity | [[evidence:root/20261108150000]] | G14 |
| M10 lease L1→L2 | [[evidence:root/20261108231249]] | G11 |
| M11 production-server | [[evidence:root/20261108231300]] | G15 smart HTTP |
| M15 inventory full CLI | [[evidence:root/20261109203000]] | G20 |
| M16 complete flag symmetry | [[evidence:root/20261109061600]] / [[evidence:root/20261109063000]] | G21 |
| M17 full G19 journeys | [[evidence:root/20261109090000]] (19/19 measured, N/A-free; supersedes TAG/DIFF N/A era) | G19 |
| M18 Pf4 aggressive ≤0.5× | [[evidence:root/20261109143000]] (11/19 claim + journey-class residuals; M17 bar green) | G19 |
| M22 residual terminal close | [[evidence:root/20261110154000]] (G8 refuse + Pf5 19/19 + server/L3/G9 terminal) | G8/G19 |
| M23 L3 claimed shape | [[evidence:root/20261109194858]] (g011-lease-productl3 dual-path unlock) | G11-L3 |
| M24 CELIR index dual-path | [[evidence:root/20261109203236]] (host advisory) + [[evidence:root/20261109204428]] (G22) + [[evidence:root/20261109205534]] (G23) + [[evidence:root/20261109205535]] (G24) + [[evidence:root/20261109205536]] (G25) | feature `celir-index` / G22–G25 |
| G28 live-private journeys | [[evidence:root/20261109212500]] (live PASS=21) + [[evidence:root/20261109213400]] (offline clones PASS; N/A=0); land tip ~`902d728` | G28 |
| M25 server transports | [[evidence:root/20261109221500]]…[[evidence:root/20261109223000]] (G29–G32) | G29–G32 |
| M26 shared-index N-way | [[evidence:root/20261110074200]] (G34) | G34 |
| Umbrella Open=0 | [[evidence:root/20261109195122]] (tip G11+G11-L3; carrying G14–G21) | G11 / G11-L3 |

## Rule

### A — Client residuals

| Item | Disposition | Normative home |
|------|-------------|----------------|
| Clean-room Git **pin-advance** | **Process closed** — follow [[sot:root/clean-room-git-pin-advance]]; current pin remains Git **2.39.5** (`verify/images.lock.json`) | Pin SoT + M9 plan |
| Credential-helper / `askpass` | **Claimed (G33)** — fill + helper + askpass + **TTY** + first-party **wincred/libsecret/osxkeychain**; forge OAuth remains AUTH/N/A | [[sot:root/credential-helper-askpass]], [[sot:root/cli-surface]], [[evidence:root/20261110224000]], [[evidence:root/20261110233000]] |
| Pc inventory honesty rows | **Closed** with M9 — implement/refuse ledger [[sot:root/cli-flag-parity-m9]]; matrix `verify/matrices/matrix-m009-clean-room-cli-parity.md` | M9 plan + G14 |
| `worktree` + `submodule` | **Closed (implement)** — leave M9 historical refuse intact; claim matrix + G16 rows **J-WORKTREE-01** / **J-SUBMODULE-01** per [[plan:root/20261109022956]]; receipt [[evidence:root/20261109114207]] | Flag ledger + journey SoT + G14/G16 |
| Alias re-smoke | **Regression** — re-prove via G14 under residual hygiene; claimed `worktree`/`submodule` under alias | G14 / alias docs |
| Same-`.git` after expansions | **Regression** — G14 + G11/G15 same-`.git` / stock-Git peer rows; no parallel store | git-compat + gates |
| `version` / `--version` | **Closed (implement)** — Sy-5 ledger row; `arks version` and `arks --version` print matching `arks version <semver>`; G21 **M16-SY5-05** | [[sot:root/cli-flag-parity-complete]], `verify/matrices/matrix-g021-cli-flag-symmetry.md` |
| Plumbing inventory beyond everyday set | **Closed M15** minimum; **M16 Sy-3** complete plumbing breadth | [[sot:root/cli-flag-parity-complete]] |
| Full CLI symmetric inventory (M9 refuse→implement) | **Closed historical M15** — Fs-1..Fs-5 G20 green; carrying [[evidence:root/20261109203000]] | M15 plan |
| Per-flag refuse leftovers after M15 | **Closed M16** — Sy-1..Sy-5 G21 green; active bar = implement\|N/A; [[evidence:root/20261109061600]] / [[evidence:root/20261109063000]] | [[plan:root/20261109043910]] |
| Client SSH transport | **Closed M16 Sy-4** — OpenSSH helper for clone/fetch/push/submodule add; server SSH command-mode is **Claimed (G30)** (not N/A) | M16 plan + arks-remote; [[sot:root/arks-server]], [[evidence:root/20261109222000]] |
| Full G19 = all G16 journeys | **Closed (M17)** — Docker G19 PASS; **19/19 measured**; `na: {}` (no TAG/DIFF timer-floor N/A); [[evidence:root/20261109090000]]; M14 frozen allow-list remains historical | [[plan:root/20261109043916]] |
| Aggressive ≤0.5× Pf4 (M18) | **Closed (M18)** — progressive claim **11/19 ≤0.5×** + journey-class residuals; M17 ≤0.8× frozen; [[evidence:root/20261109143000]] (Pf4-3 exit); Pf4-1/2 progress [[evidence:root/20261109130000]] / [[evidence:root/20261109141500]] | [[plan:root/20261109125438]] |
| Pf4 journey-class residuals (quantum / process-tax / mid) | **Closed (Pf5 aggressive_na)** — §F P6 rows; `aggressive_na` in `g019-eval.py`; reopen path [[adr:root/20261110162000]]; M17 ≤0.8× remains N/A-free measured | [[plan:root/20261110150000]], [[sot:root/clean-room-journey-compare]], [[adr:root/20261110162000]] |
| G8 live remote smoke | **Claimed (default-on when secrets)** — live when token+URL present; honest SKIP when absent; force-off via `ARKS_G8_LIVE=0`; G7 remains offline remote proof ([[adr:root/20261110220000]] supersedes default-refuse for harness policy) | [[sot:root/interop-test-matrix]], [[adr:root/20261110220000]], [[evidence:root/20261110222000]] |
| G9 partial/shallow filters | **Claimed (`--depth` / G35 + `blob:none` + `tree:0` host wire)** — smart HTTP/SSH `--depth N` + `.git/shallow` (Docker G35 **PASS**); `--filter=blob:none`/`tree:0` + promisor [[evidence:root/20261110105855]] + [[evidence:root/20261110210500]]; other filter-specs **N/A** | [[sot:root/interop-test-matrix]], [[sot:root/arks-remote]], `verify/matrices/matrix-g035-shallow-depth.md`, [[evidence:root/20261110180000]], [[evidence:root/20261110105855]], [[evidence:root/20261110210500]] |

### B — Lease residuals

| Item | Disposition | Normative home |
|------|-------------|----------------|
| L3 high-concurrency claimed shape | **Claimed (M23)** — lease+batch-apply+worktree/ref-shard unlocked [[evidence:root/20261109194858]]; M22 historical refuse preserved as history only | [[adr:root/20261109194200]] + [[plan:root/20261109194208]] + [[evidence:root/20261109194858]] |
| Shared-index N-way | **Claimed (M26 / G34 Verified)** — lease `index:shared` + `index.lock`; ≠ M24/G22/G24; historical M22 refuse ([[adr:root/20261110160000]]) superseded by unlock [[adr:root/20261110074000]] | [[adr:root/20261110074000]], [[evidence:root/20261110074200]], [[sot:root/shared-index-nway-clean-room]], lease SoT |
| Allocator helper crate identity | **Closed** — ring-internal modules; **no** dedicated allocator crate until a future ADR reopens with measured claim | [[adr:root/20261109122126]], [[sot:root/crate-architecture]] |
| Multi-agent concurrency catalog (M21 exit) | **Bare Open 0; Deferred 0.** Claimed unique **212** (G17 **145**/145; G18 **72**/72); **N/A** ×9; catalog **Refuse-closed** ×0 (+ L3 product valve). Honesty: `Deferred=0 Claimed=212 Refuse=0 N/A=9`. Does **not** reopen M12–M20; does **not** market “all races fixed” | [[sot:root/multi-agent-git-concurrency-scenarios]], [[plan:root/20261110140000]] |
| G8 live remote / CRED-NET catalog unlock | **Closed for catalog** — CRED/NET deferred IDs Claimed via Docker-local honesty (Dz-3); product G8 harness **default-on when secrets** ([[adr:root/20261110220000]]) | [[plan:root/20261110140000]], [[sot:root/interop-test-matrix]], [[adr:root/20261110220000]] |

### C — Server residuals

| Item | Disposition | Normative home |
|------|-------------|----------------|
| SSH command-mode | **Claimed (G30)** — duplex stdio pack protocol | [[sot:root/arks-server]], [[evidence:root/20261109222000]] |
| `git://` daemon | **Claimed (G32)** — [git-daemon](https://git-scm.com/docs/git-daemon); `arks daemon` | [[sot:root/arks-server]], [[evidence:root/20261109223000]] |
| Dumb HTTP product path | **Claimed (G29 server)** + **client fetch/clone Claimed** + **client dumb push Claimed (R2)** (WebDAV write; dumb-only sole server still forbidden) | [[sot:root/arks-server]], [[sot:root/arks-remote]], [gitprotocol-http](https://git-scm.com/docs/gitprotocol-http), [git-http-push](https://git-scm.com/docs/git-http-push), [[adr:root/20261110190000]], [[evidence:root/20261109221500]], [[evidence:root/20261110190000]] |
| Protocol v2 product | **Claimed (G31)** — advertise only implemented `ls-refs`/`fetch` subset | [[sot:root/arks-server]], [gitprotocol-v2](https://git-scm.com/docs/gitprotocol-v2), [[evidence:root/20261109222500]] |
| Optional HTTP Basic (in-process) | **Claimed (G36)** — product Basic when configured; forge OAuth still **Refuse** | [[sot:root/arks-server]], [[adr:root/20261109224236]], [[evidence:root/20261109224717]] |
| Server-side hooks subset | **Claimed** — honor list pre-receive / update / post-receive | [[sot:root/arks-server]] §Receive-pack githooks, [[evidence:root/20261110200000]] |
| G15 matrix | **Frozen** smart HTTP Claimed — `verify/matrices/matrix-m011-clean-room-git-server.md`; transport expansion uses **G29–G32** | matrix-m11 + matrix-g029-server-dumb-http…g032-git-daemon-wire |

### D — Public-flip hygiene

| Item | Disposition |
|------|-------------|
| Root LICENSE SPDX | **Done** — `Apache-2.0 OR MIT` |
| `SECURITY.md` | **Done** — private Advisories + pre-1.0 `main`-only |
| `.github/CODEOWNERS` | **Done** — landed; ownership hints only |
| Other public-flip owner decisions (`.spec/` publish lean, crates.io timing, Pages CELIR wording, docs↔spec links) | **Closed (owner policy recorded)** — lean defaults locked in [[sot:root/public-surface]]; not development Opens; M22 terminates prior Closed-deferred label |
| CELIR indexing-service dual-path (arks cite + enablement) | **Closed (recorded; opt-in enabled)** — catalog Current `ef6ca5af…` (tip hygiene `3a5af1ca…` retaining pack); primary cite `.agents/consumer/arks/indexing-service.md`; enablement [[adr:root/20261110052030]]; feature `celir-index` default OFF; VCS Refuse + FS sole object store retained; OID gap honest (sidecar / rebuild-from-`.git`); multi-writer = `arks-lease` — [[sot:root/storage-celir-boundary]] v0.8.1 |
| CELIR dual-path companion (sidecar / rebuild / query) | **Claimed as M24** — [[plan:root/20261109203430]]; prior waves [[plan:root/20261109202815]]; [[evidence:root/20261109203236]] host advisory; **G22 Verified** [[evidence:root/20261109204428]] (N=32/128); **G23 CELIR dual-path Git Verified** [[evidence:root/20261109205534]]; **G24 L3+index joint Verified** [[evidence:root/20261109205535]]; **G25 scale Verified** [[evidence:root/20261109205536]] (N=256/384/512); owning [[sot:root/celir-index-dual-path]] + G22/G23/G24/G25 SoTs; sidecar beside `.git`; ObjectStore Refuse retained ([[adr:root/20261109204916]]) |
| Human vs agent public vocabulary (Pages plain language; Specfold IDs precise; no false “faster than Git”) | **Closed** — [[sot:root/public-surface]] v0.3.0; human `docs/` / README audited |
| Local branch + synthetic PR-env scale (G26) | **Claimed (G26 PASS)** — N=2048 branches + 2048 PR heads + 32 workers; **not** GitHub PR throughput; live forge scale = harness only ([[adr:root/20261110161000]]) | [[sot:root/local-branch-pr-sim-clean-room]], [[plan:root/20261109205507]], [[evidence:root/20261109205847]] |
| arks smart-HTTP branch + synthetic PR contention (G27) | **Claimed (G27 PASS)** — N=2048 branches + 2048 PR heads + 8 agents via arks smart HTTP; **not** GitHub UX/Checks/merge-queue; **not** live forge throughput | [[sot:root/arks-smart-http-pr-contention-clean-room]], [[plan:root/20261109210427]], [[evidence:root/20261109210427]] |
| GitHub live PR/forge latency & rate-limit capacity | **Closed (harness + opt-in)** — [[adr:root/20261110161000]]; `verify/matrices/matrix-g008-live-scale.md` + `run-g008-live-scale.sh`; no invented latency; ≠ G28 functional claim | [[sot:root/interop-test-matrix]], [[adr:root/20261110151000]], [[adr:root/20261110161000]] |
| Live private forge **functional journeys** (G28) | **Claimed (G28 PASS)** — live private GitHub happy paths (PASS=21; [[evidence:root/20261109212500]]) + offline clone closure **J-CLONE-LOCAL-01** / **J-CLONE-PATH-01** PASS ([[evidence:root/20261109213400]]); checklist N/A=0; ≠ throughput/contention; `gpgsig` **read** Claimed ([[evidence:root/20261110170000]]); SSH create/verify ([[evidence:root/20261110073800]]); user-authorized opt-in; **ephemeral repos deleted** (cleanup Done) | [[sot:root/g028-live-private-arks-live-private-arks-journeys]], [[plan:root/20261109212020]], [[evidence:root/20261109212500]], [[evidence:root/20261109213400]] |

### E — Hygiene (Main)

| Item | Disposition |
|------|-------------|
| Full regression G14 + G11 + G15 + G7 (+ prior embedded) | Required for residual-close evidence |
| `spec check` | Clean |
| Public honesty | README / `docs/` match bars + receipts; human vs agent vocabulary per [[sot:root/public-surface]]; no “faster than Git” marketing; M16 active alias / M17 evidence-linked |
| Bug simulation notes | Per-gate BUGSIM as exercised |
| Residual evidence receipt | [[evidence:root/20261108233129]] (historical hygiene); M17 carrying [[evidence:root/20261109090000]]; development residual close [[evidence:root/20261109122512]] |

### Development Open register (normative empty)

| Item | Status |
|------|--------|
| *(none)* | **No unfinished development Opens** after v0.6.0 residual close |

### F — Future-work inventory (38 terminal rows; not development Opens)

Every row is **Claimed / Refuse-closed / N/A / Closed** — **no bare Deferred**. They are **not** bare Opens and MUST NOT be re-litigated as “M22 unfinished.” Reopen only via superseding ADR / SoT amendment + gate. Priority **P0–P6** matches agent work-queue partitioning (documentation streams β–ζ).

| P | ID | Disposition | Normative cite | Reopen path (if any) |
|---|---|-------------|----------------|----------------------|
| **P0** | **FW-G8-DEFAULT** | **Claimed (default-on when secrets)** — live when token+URL; SKIP when absent | [[adr:root/20261110220000]], [[evidence:root/20261110222000]] | Force-off via `ARKS_G8_LIVE=0`; live PASS needs secrets + receipt |
| **P0** | **FW-GH-LIVE-SCALE** | **Claimed (harness ready)** — measure when secrets; no-measurement SKIP when absent | [[adr:root/20261110221000]], `verify/matrices/matrix-g008-live-scale.md`, [[evidence:root/20261110223000]] | No public SLO without gate; never invent latency |
| **P1** | **FW-SHARED-IDX** | **Claimed (M26 / G34)** — shared-index N-way contention | [[adr:root/20261110074000]], [[evidence:root/20261110074200]], [[plan:root/20261110074000]], `verify/matrices/matrix-g034-shared-index-contention.md` | Re-verify G34 after shared-index protocol change |
| **P1** | **FW-G9** | **Claimed (--depth / G35 + blob:none + tree:0 host wire)** — shallow clone/fetch Docker G35 **PASS**; `--filter=blob:none`/`tree:0` [[evidence:root/20261110105855]] + [[evidence:root/20261110210500]]; other filter-specs N/A | [[sot:root/arks-remote]], [[sot:root/interop-test-matrix]], `verify/matrices/matrix-g035-shallow-depth.md`, [[evidence:root/20261110180000]], [[evidence:root/20261110105855]], [[evidence:root/20261110210500]] | Symmetry e2e twin **S6**/G39; re-verify after filter/depth wire change |
| **P1** | **FW-CRED-ASKPASS** | **Claimed (G33)** — helper / askpass / TTY / first-party OS wallets | [[sot:root/credential-helper-askpass]], [[evidence:root/20261110224000]], [[evidence:root/20261110233000]] | Re-verify G33 after wire change |
| **P1** | **FW-AUTH-PROXY** | **Claimed (product Basic G36)** + **Refuse-closed (forge OAuth)** — in-process Basic when configured | [[adr:root/20261109224236]], [[evidence:root/20261109224717]], [[sot:root/arks-server]] | Re-verify G36 after auth wire change |
| **P1** | **FW-PF5-NA** | **Claimed 19/19** after hardlink sole G19 [[evidence:root/20261111032000]]: M17 19/19; ≤0.5× claimed=19; ALIAS **0.4926 Claimed**; REVERT **0.4227**. Prior G7 aggressive_na archival [[evidence:root/20261111025200]]. **No thrash**. Historical **8/8** at `79fc9bf` archival | [[sot:root/pf5-journey-seed-amortization]], [[adr:root/20261111031600]], [[evidence:root/20261111032000]], [[evidence:root/20261111025200]], [[evidence:root/20261110142500]] | Regress G19 after peer/pack-reuse / sha1cd / amortize / alias shim change |
| **P2** | **FW-SRV-SSH** | **Claimed (G30)** — server SSH command-mode | [[adr:root/20261109220230]], [[evidence:root/20261109222000]] | Re-verify G30 after wire change |
| **P2** | **FW-SRV-GIT** | **Claimed (G32)** — `git://` daemon | [[adr:root/20261109220230]], [[evidence:root/20261109223000]] | Re-verify G32 after wire change |
| **P2** | **FW-SRV-DUMB** | **Claimed (G29)** — dumb HTTP additive path | [[adr:root/20261109220230]], [[evidence:root/20261109221500]] | Re-verify G29 after wire change |
| **P2** | **FW-SRV-V2** | **Claimed (G31)** — protocol v2 advertisement | [[adr:root/20261109220230]], [[evidence:root/20261109222500]] | Re-verify G31 after wire change |
| **P2** | **FW-SRV-HOOKS** | **Claimed** — honor list `pre-receive` / `update` / `post-receive` / `post-update` + quarantine migrate | [[sot:root/arks-server]] §Receive-pack githooks, [[evidence:root/20261110200000]] | Re-verify G15 M11-C14 after hook/quarantine change |
| **P2** | **FW-PIN** | **Closed (process)** — pin advance hygiene; pin **2.39.5** | [[sot:root/clean-room-git-pin-advance]], `verify/images.lock.json` | Deliberate pin bump + G14-class re-prove |
| **P2** | **FW-SLO** | **Closed** — M17 ≤0.8× + M18/Pf5 evidence; no new host SLOs | [[evidence:root/20261109090000]], [[evidence:root/20261109143000]], [[sot:root/clean-room-journey-compare]] | Clean-room gate only; no laptop marketing SLO |
| **P3** | **FW-CELIR-OS** | **Refuse-closed (terminal)** — CELIR ObjectStore | [[sot:root/storage-celir-boundary]], [[adr:root/20261110052030]], [[adr:root/20261109204916]] | ObjectStore reverse ADR (forbidden product path) |
| **P4** | **FW-GUI** | **Claimed (FS7 handoff)** — external `gui`/`gitk`/`citool` refuse+reason; not embed Tcl/Tk | [[research:root/20261110212414]], [[adr:root/20261110213500]] | Re-verify after GUI wire change |
| **P4** | **FW-SCALAR** | **Claimed (FS6 refuse)** — `scalar` refuse with reason (≠ core git porcelain) | [[research:root/20261110212414]] | Documented closest |
| **P4** | **FW-LFS** | **Trimmed (product)** — in-process LFS client/server/`info/lfs` removed; stock `filter.*.smudge` + external `git-lfs` only | [[research:root/20261111035700]], tip `9df7fb9`, [[sot:root/product-intent]] | Do not re-Claim LFS productization |
| **P4** | **FW-FORGE-B** | **Design under `arks hub` (not Claimed)** — host PR/Checks UX is scope (B); same `arks` binary + internal `arks-hub`; native API; never spawn `gh`/`git`; **no** separate hub release | [[sot:root/arks-hub-product-intent]], [[sot:root/remote-host-compatibility-surface]] scope (B) | Claimed only with hub evidence |
| **P5** | **FW-CAT-FS-007** | **Impossible (full sync)** + **Trimmed product CLI** — `arks host-env` removed; multi-device cloud-sync remains Impossible | Catalog FS-007, [[research:root/20261111035700]], tip `9df7fb9` | Multi-device races still Impossible |
| **P5** | **FW-CAT-FS-014** | **Impossible (bypass)** + **Claimed(detect+errno)** — sysfs SELinux/AppArmor detect; Docker `fs014-lsm-advisory` | Catalog FS-014, [[evidence:root/20261111003444]] | Host policy bypass Impossible |
| **P5** | **FW-CAT-HOOK-008** | **Claimed (FS8 closest)** — `proc-receive.lock` concurrent refuse; FS2 protocol Claimed | Catalog HOOK-008, [[adr:root/20261110212500]] | Full race-freedom without hostile gate still non-claim |
| **P5** | **FW-CAT-MAINT-004** | **Claimed (FS5 closest)** — `update-server-info` refuses under `info/refs.lock` | Catalog MAINT-004 | Re-verify after server-info wire change |
| **P5** | **FW-CAT-MISC-006** | **Claimed (FS6)** — shadow refs as ordinary refs under `refs/` | Catalog MISC-006 | Forge UX still out |
| **P5** | **FW-CAT-NET-006** | **Claimed (FS5/NET-006 + G3)** — HTTP(S)+local `--bundle-uri`; lock + Content-Length refuse; server v2 **`bundle-uri` advertise + command** | Catalog NET-006, [[adr:root/20261110214000]], [[evidence:root/20261110214100]], [[evidence:root/20261111025300]] | CDN/scale SLOs still non-claim |
| **P5** | **FW-CAT-OBJ-004** | **Claimed** — fsck OidMismatch + **sha1cd detect+refuse** + **sha256 porcelain** + **mixed-format remote refuse**; crypto SHA-1 collision immunity **Impossible** | Catalog OBJ-004, [[adr:root/20261110223000]], [[evidence:root/20261110223500]], [[adr:root/20261110214800]], [[adr:root/20261111031600]], [[evidence:root/20261110214830]] | Cross-hash OID rewrite / mixed remotes product still non-claim |
| **P5** | **FW-CAT-REF-017** | **Claimed (FS5 closest)** — `arks replace` + `replace.lock` refuse | Catalog REF-017, [git-replace](https://git-scm.com/docs/git-replace) | Re-verify after replace wire change |
| **P5** | **FW-CAT-WIN-003** | **Impossible (hydration)** + **Claimed(detect+errno honesty)** — CfAPI `RECALL_ON_DATA_ACCESS` / OneDrive path via `arks-hostfs` (skip-if-absent); DriveFS G: separated | Catalog WIN-003, [[evidence:root/20261110223500]], [[evidence:root/20261111003444]], [[research:root/20261110222000]] | G: DriveFS ≠ OneDrive placeholders |
| **P6** | **FW-PF5-J-BRANCH** | **Claimed (≤0.5×)** — J-BRANCH-01 (under umbrella **8/8**) | [[sot:root/pf5-journey-seed-amortization]], [[evidence:root/20261110224500]], [[evidence:root/20261110142500]], [[adr:root/20261110162000]] | G19 regress after amortization change |
| **P6** | **FW-PF5-J-BUNDLE** | **Claimed (≤0.5×)** — J-BUNDLE-01 (under umbrella **8/8**) | Same | Same |
| **P6** | **FW-PF5-J-RM-MV** | **Claimed (≤0.5×)** — J-RM-MV-01 (under umbrella **8/8**) | [[sot:root/pf5-journey-seed-amortization]], [[evidence:root/20261111001400]], [[evidence:root/20261110142500]], [[adr:root/20261110162000]] | G19 regress after amortization change |
| **P6** | **FW-PF5-J-STASH** | **Claimed (≤0.5×)** — J-STASH-01 (under umbrella **8/8**) | [[sot:root/pf5-journey-seed-amortization]], [[evidence:root/20261110234600]], [[evidence:root/20261110142500]] | G19 regress after amortization change |
| **P6** | **FW-PF5-J-LOCAL** | **Claimed (≤0.5×)** — J-LOCAL-01 (under umbrella **8/8**) | [[sot:root/pf5-journey-seed-amortization]], [[evidence:root/20261110224500]], [[evidence:root/20261110234600]], [[evidence:root/20261110142500]] | G19 regress after amortization change |
| **P6** | **FW-PF5-J-ALIAS** | **Claimed (≤0.5×)** — sole G19 **0.4926** after hardlink/`exec` + no PATH rewrite ([[evidence:root/20261111032000]]; prior aggressive_na **0.5489** [[evidence:root/20261111025200]]) | [[sot:root/pf5-journey-seed-amortization]], [[adr:root/20261111031600]], [[evidence:root/20261111032000]], [[evidence:root/20261111025200]], [[evidence:root/20261110142500]] | G19 regress after shim/amortize change; no thrash |
| **P6** | **FW-PF5-J-REVERT** | **Claimed (≤0.5×)** — tip G7 sole G19 **0.4943** ([[evidence:root/20261111025200]]); prior unblock **0.5000** with `PF5_REVERT_AMORTIZE_COMMITS=20` ([[evidence:root/20261111015154]]); U1 0.5417@12 archival | [[sot:root/pf5-journey-seed-amortization]], [[evidence:root/20261111025200]], [[evidence:root/20261111015154]], [[evidence:root/20261111003444]], [[evidence:root/20261110224814]] | G19 regress after amortize change |
| **P6** | **FW-PF5-J-CLONE-LOCAL** | **Claimed (≤0.5×)** — J-CLONE-LOCAL-01 flipped; primary [[evidence:root/20261110142500]] (LOCAL **0.1579** @ `79fc9bf`; PATH **0.1739** PASS; product `arks serve-http` + pack reuse); prior carrying [[evidence:root/20261110131600]]; aggressive_na packs historical | [[adr:root/20261110162000]], [[evidence:root/20261110142500]], [[evidence:root/20261110131600]], [[sot:root/pf5-journey-seed-amortization]], [[evidence:root/20261111001400]] | G19 regress after peer/pack-reuse change |
| **P6** | **FW-PF5-J-WORKTREE** | **Claimed (≤0.5×)** — J-WORKTREE-01 (under umbrella **8/8**) | [[sot:root/pf5-journey-seed-amortization]], [[evidence:root/20261110234600]], [[evidence:root/20261110142500]] | G19 regress after amortization change |
| **P0** | **FW-GPGSIG** | **Claimed (SSH + OpenPGP + X.509 + tag -s)** — `gpgsig` read; SSH **G37**; OpenPGP/`tag -s`/`verify-tag` + X.509 program spawn **G38** | [[sot:root/arks-object]] v0.1.5, [gitformat-signature](https://git-scm.com/docs/gitformat-signature), [[evidence:root/20261110073800]] (SSH), [[evidence:root/20261110110800]] (G38) | Re-verify G37/G38 after crypto wire change |
| **P0** | **FW-SUBMOD-HTTPS** | **Claimed** — `http(s)://` `submodule add` via `arks-remote` `clone_http` | [[sot:root/clean-room-journey-compare]] J-SUBMODULE-01, [[evidence:root/20261110170000]] (G7), [git-submodule(1)](https://git-scm.com/docs/git-submodule) | G28 live HTTPS peer when opt-in |

L3 ship is **M23** under [[adr:root/20261109194200]]. M21 catalog **Deferred=0** ([[plan:root/20261110140000]]). M22 terminal close ([[plan:root/20261110150000]]). CELIR index dual-path is **M24** ([[plan:root/20261109203430]]). **Closed-deferred forbidden** as lasting disposition.

### Post-terminal next-queue (reopen-only; not development Opens)

After PF5 **Claimed 8/8** ([[evidence:root/20261110142500]] primary at `79fc9bf`; prior [[evidence:root/20261110131600]]) and §F **38/38** terminal, **no bare development Opens remain**. Historical product-completion ladder **L0–L8 + L_final Done** ([[research:root/20261110095142]]; **L9** closed owner policy). **Expanded git↔arks symmetry DoD** is ordered in [[research:root/20261110105436]] (**S0–S_final**): reopen implementable honest-N/A residuals via SoT amend + wire + evidence — **not** invent Claimed/PASS. **S0** inventory lock **Done**. **S1** / **R3** `blob:none` **Claimed (host wire)** ([[evidence:root/20261110105855]]). **S2** / **R4** OpenPGP / X.509 / `tag -s` **Claimed (G38)** ([[evidence:root/20261110110800]]). **S3** dumb HTTP push deletes **Claimed (host wire)** ([[evidence:root/20261110112600]]). **S4** interactive TUI **honest defer** ([[evidence:root/20261110113200]]). **S5** live forge **secrets-block** ([[evidence:root/20261110113300]]). **S6** symmetry e2e **PASS** ([[evidence:root/20261110114000]]). **S7** / **S_final Done** ([[evidence:root/20261110114100]]). **L9** stays closed. Do **not** invent Claimed/PASS. Do **not** restart sole G19 remasure unless carrying honesty evidence is missing/broken. Tip CI host lint/test remains advisory vs Docker gates.

| Priority | Candidate | Disposition today | Reopen requires |
|----------|-----------|-------------------|-----------------|
| — | §F P0–P6 rows | All **Claimed / Refuse / N/A / Closed** | Superseding ADR + SoT amend + gate re-verify (row reopen path) |
| — | Historical ladder L0–L9 | **Done** (L9 closed) in [[research:root/20261110095142]] | Historical; do not reopen L9 |
| — | Symmetry ladder S0–S_final | Normative expanded DoD in [[research:root/20261110105436]] (**S0–S_final Done**; **L9** closed) | Keep evidence current; SoT amend before Claimed |
| R1 | Client `git://` clone/fetch/push | **Claimed (host wire)** — [[sot:root/arks-remote]] v0.5.1; clone/fetch/push vs `arks daemon`; ladder **L2** | Optional clean-room matrix twin of G32 client lane; re-verify after wire change |
| R2 | Dumb HTTP **push** | **Claimed (host wire)** — WebDAV write + **S3 deletes** [[adr:root/20261110190000]] + [[adr:root/20261110112600]]; [[sot:root/arks-remote]] v0.5.4 + [[sot:root/arks-server]] v0.3.2; ladder **L3** + **S3** | Optional clean-room twin; re-verify after DAV/wire change |
| R3 | Partial-clone filter / promisor | **Claimed (host wire + S6 G39 + O2)** — `--filter=blob:none` + `--filter=tree:0` + promisor [[adr:root/20261110105547]] + [[adr:root/20261110210000]] + [[evidence:root/20261110105855]] + [[evidence:root/20261110210500]]; other filter-specs still refuse | Re-verify G39 after filter wire change |
| R4 | OpenPGP / X.509 / `tag -s` | **Claimed (G38)** — OpenPGP commit/tag + stock verify; X.509 via configured program [[adr:root/20261110110450]] + [[evidence:root/20261110110800]]; symmetry **S2 Done** | Re-verify G38 after crypto wire change |
| R5 | Live forge scale SLO / G8 live PASS | Harness **Claimed**; **O1 secrets hard-block** (re-probed secrets=0; [[evidence:root/20261110210200]]); local-forge wire substitute `g008-local-forge` (**≠** GitHub live PASS); remasure needs secrets+receipt | Secrets + receipt for live samples; local `claimed-wire` ≠ live PASS; never invent latency |
| R6 | Pin advance (Git **2.39.5**) | Process-closed | Deliberate pin bump + G14-class re-prove |
| — | Optional O-ladder O1–O_final | Historical Done in [[research:root/20261110205403]]; superseded for feature DoD by Full Symmetry ladder | Keep evidence current |
| — | Full Symmetry ladder FS0–FS_final | **DoD met** [[research:root/20261110212414]]; **FS0–FS_final Done**; harness reopen closed; LFS **network/server** + **sha256 porcelain** Claimed [[adr:root/20261110214800]] + [[evidence:root/20261110214830]]; remaining = documented Impossible only (crypto collision immunity, forge Checks/PR UX, host FS-007/FS-014/WIN-003, **L9**, live G8); G8 secrets operator | Keep evidence current |
| — | Tip currency / operator “fully complete?” | **PARTIAL** — FS_final **YES**; Pf5 **19/19 ≤0.5× Claimed** [[evidence:root/20261111032000]]; live G8 latency secrets=0 [[evidence:root/20261111032001]] + **L9** Blocked; crypto immunity Impossible; forge/host-env/LFS-product **trimmed**; hub = `arks hub` design; **clean-room 3-pillar DoD** SoT ([[sot:root/verify-cleanroom-symmetry-matrix]]; tip ≥ `053e5ef`; CLI pin Missing=0; smoke ≠ Claimed) | No invent PASS; no G19 stampede; no unauthorized L9; single release = `arks` |
| — | Clean-room symmetry DoD (git / daemon / hub) | **SoT tip `c297e34` + local refresh** — umbrella [[sot:root/verify-cleanroom-symmetry-matrix]]; **CLI pin Missing=0** (host 2.56 skew Partial only); **daemon** former Missing~25 → CSV Missing=0 (Partial+Refuse); **hub v1** Partial stubs + gh 2.103.3 new groups inventory; pin judge `git 2.39.5` | Claimed only with `ARKS_DOD_STRICT=1` + evidence; host skew ≠ pin; no invent PASS |
| — | Release symmetry precheck (ops/CI) | **Wired** [[sot:root/release-symmetry-precheck]] — GHA `release-symmetry.yml` + `release.yml` `needs: symmetry-precheck`; pin Missing=0 + untracked leaves **block** `dist`; Partial + hub v1 Missing **advisory** by default; latest inventory continuity ≠ pin Claimed | No invent PASS from green gate; flip hub Missing block only when hub bar lands |
| — | arks-only self-host server (LAN/WAN) | **Design** [[sot:root/arks-self-host-server]] — related hosting under [[sot:root/verify-cleanroom-symmetry-matrix]] (`dod-self-host` scaffold); ≠ daemon flag parity; wire G15/G29–G32/G36 already Claimed separately | Hosting e2e Claimed only with clean-room evidence; no invent PASS |
| — | CLI personality / output+exit contract | **Design + sample+expand Claimed** — [[sot:root/arks-cli-personality-factory]] + [[sot:root/arks-cli-output-exit-contract]]; sample [[evidence:root/20261111014608-personality-dropin]]; expand [[evidence:root/20261111020515-personality-expand]]; cli survey Missing=0; per-cmd peer tables Partial | ExitCodePolicy units ≠ drop-in Claimed; no invent PASS |
| — | Claimed residual gates (post-Partial) | **Active** [[sot:root/arks-claimed-residual-gates]] — **B1/B2/B3/B4/B5/B5+/B7(HTTP+SSH+forge)/B9 Claimed** with Docker receipts ([[evidence:root/20261111025206-daemon-b5-plus-syslog]] for B5+). | No invent Claimed; remaining IW rows via honesty leftovers / Partial honesty |
| — | Remaining implement wave (research-gated) | **Active** [[sot:root/arks-remaining-implement-wave]] + [[research:root/20261111090827]] — **IW-B5 / IW-B7-SSH / IW-B7-FORGE / IW-SSH-LABEL Claimed**; Partial leftovers deferred to next row | Spec-first; Partial honesty over fake Claimed |
| — | Partial residual wave (research-gated) | **Historical** [[sot:root/arks-partial-residual-wave]] + [[research:root/20261111093543]] — FORGE-SERVE / HUB-FLAT / PERSONALITY sample Claimed; daemon flags Partial; B5+ superseded Claimed by honesty leftovers | Prefer Partial honesty; no invent Claimed |
| — | Verify Docker e2e claim wave | **Historical** [[sot:root/arks-verify-docker-e2e-claim-wave]] — FORGE-SERVE + HUB-FLAT + PERSONALITY sample Claimed; B5+ superseded by honesty leftovers collector Claimed | Spec-first; Claimed only with `.spec/evidence/` |
| — | Remaining incompletes wave | **Closed** [[sot:root/arks-remaining-incompletes-wave]] + [[research:root/20261111105924]] — HUB-FULL + PERSONALITY-FULL Claimed; B5+ superseded Claimed by honesty leftovers | Keep Partial honesty for non-mutate hub leaves + CLI survey Partial×117 |
| — | Honesty leftovers wave | **Closed** [[sot:root/arks-honesty-leftovers-wave]] + [[research:root/20261111114312]] — **IW-HUB-MUTATE Claimed** ([[evidence:root/20261111025142-hub-mutate-subset]]); **IW-PERSONALITY-EXIT Claimed** named rows ([[evidence:root/20261111025151-personality-exit-tables]]); **IW-B5-PLUS Claimed** ([[evidence:root/20261111025206-daemon-b5-plus-syslog]]); Actions/assets/checkout + CLI Partial×117 + daemon flag deepen reopened under in-scope leftovers | Prefer Partial honesty until in-scope Claimed receipts |
| — | In-scope leftovers wave | **Active deepen** [[sot:root/arks-in-scope-leftovers-wave]] + [[research:root/20261111123556]] + [[research:root/20261111133600]] — tip `5ae7ad9` ACTIONS/ASSETS/CHECKOUT/UX/survey/flags; **IW-HUB-STUBS** Claimed subset + Stay Partial remainder; survey2 named rows; upload-archive enable+ustar Parity; git:// client pointer Stay Partial; SysV/L9/G8/Impossible/later/OUT untouched | Claimed only with Docker receipts; no invent PASS |
| — | Verify directory conventions | **Design** [[sot:root/verify-directory-conventions]] + [[research:root/20261111082440]] — tree contract, `gN` vs `dod-*` naming, `verify/out/` vs `gates/*/out/`, SKIP vs `ARKS_DOD_STRICT=1`, pin Git **2.39.5** normative, residual registration checklist | Ops/harness only; does not invent product Claimed; coordinate harness edits with in-flight B-gate owners |

## Why

After M0–M17 claimed exits, agents otherwise re-litigate closed bars or treat policy-deferred gates as unfinished development. A single residual register freezes implement vs refuse-close vs Closed-deferred and ties public honesty to evidence ids.

## Verification

1. `spec check` clean for this SoT and linked residual updates.
2. Every row above is either proven by carrying evidence, refuse-closed / N/A in owning SoT/ADR, or process-closed — **no bare development Opens** and **no Closed-deferred work items** after M22 ([[plan:root/20261110150000]] / [[evidence:root/20261110154000]]).
3. Public copy MAY cite G29–G32 / G33 / G34 / G35 / G36 / G37 / **G38** and **R3 `blob:none`** / **R4 OpenPGP·X.509·tag-s** / **`sparse:oid=<hex>`** only with matching Claimed evidence ([[evidence:root/20261110105855]], [[evidence:root/20261110110800]], [[adr:root/20261111031600]]); MUST keep unlisted filter-specs **N/A**/refuse, forge OAuth **Refuse**, and PF5 honesty current (ALIAS after one sole G19; historical **8/8** [[evidence:root/20261110142500]] archival). G33 MAY claim TTY + first-party OS wallets; MUST NOT claim forge OAuth under CRED.
4. M17 cites [[evidence:root/20261109090000]] (not the TAG/DIFF N/A-era receipt alone).
