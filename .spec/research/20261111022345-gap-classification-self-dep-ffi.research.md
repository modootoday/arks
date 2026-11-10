---
id: research:root/20261111022345
title: Remaining gaps — self-implement vs dependency packages vs FFI/CLI
status: active
created: 2026-11-10T17:23:45Z
updated: 2026-11-10T17:23:45Z
tags:
  [
    gap-classification,
    self-implement,
    dependency-packages,
    ffi,
    external-cli,
    exotic-filters,
    force-with-lease,
    lfs-tus,
    bundle-uri,
    mixed-hash,
    scalar,
    gui,
    forge-ux,
    host-env,
    git-pin,
    pf5-alias,
    g008-live-forge-smoke-live,
    l9,
    sha1-immunity,
    celir-objectstore,
    multi-writer,
  ]
verify: manual
rel:
  depends_on:
    - research:root/20261111012840
    - research:root/20261110222000
    - research:root/20261111001816
    - research:root/20261111003444
    - sot:root/program-roadmap
    - sot:root/production-residual-register
    - sot:root/pf5-journey-seed-amortization
    - sot:root/public-surface
    - evidence:root/20261111015154
    - evidence:root/20261111003444
    - adr:root/20261110162000
    - adr:root/20261110214800
---

# Remaining gaps — self-implement vs dependency packages vs FFI/CLI

## Question

At tip ≥ **`ade4040`**, with **FS_final YES** and operator honesty **PARTIAL**, classify every remaining non-casual gap into:

1. **Self-implement** — in-tree / own code primarily required  
2. **Dependency packages** — crates/npm/etc. largely unblock a Claimed slice  
3. **FFI or external CLI** — host binary / OS API / Actions secrets / subprocess required  

…with Specfold honesty (what can become **Claimed** vs stays **Impossible** / **Non-goal** / **Operator-only**), without inventing PASS, G19 thrash, or L9 publish.

## Method

1. Reconcile tip table ([[sot:root/program-roadmap]] v0.15.4), residual §F ([[sot:root/production-residual-register]] v0.9.9), `docs/completion.md` / `docs/git-vs-arks.md` / `docs/limits.md`.  
2. Fold prior research: [[research:root/20261111012840]], [[research:root/20261110222000]], [[research:root/20261111001816]].  
3. Inventory feature asymmetries still listed as refuse / non-claim / Impossible / Blocked / Non-goal.  
4. For each gap: bucket (1/2/3 or **hybrid**), why, concrete packages/CLIs/FFI if 2/3, effort, honesty outcome, recommended next step.  
5. Rank **best ROI if deps/FFI acceptance is allowed**. Analysis only — **no** G19 remasure, **no** L9 publish, **no** invent live G8 PASS.

## Tip honesty baseline

| Item | Value |
|------|-------|
| Tip | ≥ **`ade4040`** (*Execute external-deps unblock: REVERT ≤0.5× sole G19…*) |
| FS_final | **YES** |
| Operator one-word | **PARTIAL** |
| Pf5 | REVERT **0.5000 Claimed**; ALIAS **0.5546** `aggressive_na`; claimed=18; M17 **19/19** ([[evidence:root/20261111015154]]) |
| `g008-live-forge-smoke-live` secrets | **0** (last probe in [[research:root/20261111012840]]) |
| L9 | Owner-closed; reopen pack only |
| sha1cd | Detect+refuse **Claimed**; immunity **Impossible** |
| Host-env | Detect + opt-in refuse / LSM detect **Claimed**; full sync/hydration/bypass **Impossible** |
| Forge UX | Claimed-closest `arks forge` → `gh`; full product **Non-goal** |

### Bucket vocabulary

| Bucket | Meaning |
|--------|---------|
| **1 Self-implement** | Protocol / porcelain / PATH redesign in arks crates; crates may help but are not the unblock |
| **2 Dependency packages** | Adding/wrapping a library largely closes a Claimed slice after SoT + evidence |
| **3 FFI / external CLI** | Must call host OS API, spawn `gh`/`git-lfs`/`scalar`/`cargo`, Actions secrets, or similar |
| **Hybrid** | Meaningful work in ≥2 buckets; table names the dominant + secondary |
| **Hold / Impossible** | No honest Claimed upgrade path without SoT reopen that redefines product or math |

Effort: **S** ≤1–2 eng-days · **M** ~1 sprint slice · **L** multi-sprint / policy · **Ops** human secrets/owner only.

---

## Per-gap classification tables

### A — Feature asymmetries (post-FS_final non-claims)

#### A1. Exotic partial-clone filters

| Field | Content |
|-------|---------|
| **Tip status** | Claimed: `blob:none`, `tree:0`, `tree:<n>`, `blob:limit=<n>[kmg]`; exotic refuse (`sparse:oid=…`, `object:type=…`, `combine:…`, multi-`--filter`) |
| **Bucket** | **1 Self-implement** (dominant); **hybrid→2** only if adopting a filter-engine crate (none normative today; workspace forbids `gix`) |
| **Why** | Filter-spec parse + pack-objects omit + promisor bookkeeping is in-tree protocol work already partially shipped (FS4). Remaining specs are the same class of code — not a crate gap. |
| **Packages / CLI / FFI** | Optional reference: stock Git as oracle in Docker; **do not** pull `gix`. No FFI required. |
| **Effort** | **M–L** per family (`sparse:oid` hardest; `object:type` / simple `combine:` of claimed children medium) |
| **Specfold honesty** | SoT amend + host wire + G39-class evidence → **Claimed** per filter family. Until then keep **N/A / refuse-with-reason**. FS_final already honest. |
| **Recommended next** | Hold unless product needs sparse-OID clones; if reopen, implement `object:type` first (smallest), then `combine:` of already-Claimed children, then `sparse:oid`. |

#### A2. Dumb HTTP `push --force-with-lease`

| Field | Content |
|-------|---------|
| **Tip status** | Smart-pack lease **Claimed**; dumb HTTP lease **not** claimed (`docs/limits.md`) |
| **Bucket** | **1 Self-implement** |
| **Why** | WebDAV dumb push has no pack negotiation channel for lease tokens; need product-defined compare-and-swap over LOCK/PUT/MOVE (or honest refuse forever). Stock Apache twin is already G41 for write subset — lease is protocol design, not a dep. |
| **Packages / CLI / FFI** | None required. Oracle: stock `git` + Apache `mod_dav` in clean-room. |
| **Effort** | **M** (design + wire + G41-adjacent matrix) |
| **Specfold honesty** | Implementable → **Claimed** with receipt **or** keep documented asymmetry (smart-only lease). Do **not** invent lease PASS on dumb without CAS semantics. |
| **Recommended next** | Low ROI vs smart-pack; document “smart-only” unless a dumb-only peer forces reopen. |

#### A3. LFS tus / locks / multi-tenant S3

| Field | Content |
|-------|---------|
| **Tip status** | Batch/basic network + `serve-http` `info/lfs` **Claimed**; tus / LFS locks product / multi-tenant S3 **non-claim** (FW-LFS reopen path) |
| **Bucket** | **Hybrid 2+3** (dominant **2** for locks API; **3** for S3 transfer agents / tus servers) |
| **Why** | Batch/basic already in-tree. Locks are HTTP JSON endpoints; tus is a resumable transfer adapter; S3 is typically a **custom transfer agent** or presigned URL backend — external to core revision engine. |
| **Packages / CLI / FFI** | Crates: `git-lfs-api` (batch + **locks** client), `git-lfs-transfer` (**basic only** today — tus/custom **not** implemented upstream). CLIs: stock `git-lfs`, AWS `git-remote-s3` / `git-lfs-s3` custom transfer, `git-remote-object-store` LFS agent. Server: tusd / forge LFS + S3. |
| **Effort** | Locks client/server subset **M**; tus adapter **M–L**; multi-tenant S3 product **L** + ops |
| **Specfold honesty** | Thin wrap / in-process locks subset → **Claimed** slice after SoT + evidence. Full “S3 LFS product / tus parity with git-lfs” stays **Partial** unless scope reopen. Handoff to `git-lfs` for exotic argv already closest. |
| **Recommended next** | If product wants locks: implement `/locks` honor on `serve-http` + client verify (**2** with `git-lfs-api` types or hand-rolled). Prefer **handoff** for tus/S3 agents (**3**) over reimplementing AWS agent. |

#### A4. Server `bundle-uri` advertise

| Field | Content |
|-------|---------|
| **Tip status** | Client `--bundle-uri` HTTP(S)+local **Claimed** (NET-006); **server advertise** still non-claim |
| **Bucket** | **1 Self-implement** |
| **Why** | Protocol v2 / info capability advertisement + serving bundle bytes is in-tree `serve-http` / upload-pack work; client path already exists. |
| **Packages / CLI / FFI** | None. Optional CDN front is ops, not a crate unblock. |
| **Effort** | **M** |
| **Specfold honesty** | Advertise + fetch path + lock/truncation honesty → **Claimed** server slice. CDN/scale SLOs remain separate (G8-class). |
| **Recommended next** | Medium ROI for mirror/bootstrap peers; SoT amend NET-006 server row → implement advertise → evidence. |

#### A5. Mixed sha1 ↔ sha256 remotes

| Field | Content |
|-------|---------|
| **Tip status** | sha256 porcelain **Claimed**; mixed-hash remotes **non-claim** (OBJ-004 reopen note) |
| **Bucket** | **1 Self-implement** (dominant); **Hold / near-Impossible** as full product Claimed without major Git-compat research |
| **Why** | Cross-hash-function fetch/push needs object rewriting or dual-OID maps — not solved by adding a hasher crate (sha1cd/sha256 already present). Stock Git itself treats this as an advanced/compat edge. |
| **Packages / CLI / FFI** | Hash crates already in-tree; no external CLI unblocks mixed remotes. |
| **Effort** | **L** (research + wire + migration story) |
| **Specfold honesty** | Keep **non-claim / refuse**. Detect mismatched `extensions.objectFormat` and refuse honestly is the Claimable slice today. Full mixed remotes ≠ Claimed without primary SoT + long ladder. |
| **Recommended next** | Docs + refuse honesty only; do not schedule as sprint work. |

#### A6. `scalar`

| Field | Content |
|-------|---------|
| **Tip status** | **Claimed (FS6 refuse)** — not core git porcelain |
| **Bucket** | **3 FFI / external CLI** (if product wants closest); else **Hold** |
| **Why** | `scalar` is a Microsoft companion tool (VFS/sparse enrollment), not a filter-spec. Embedding equals reimplementing that product. |
| **Packages / CLI / FFI** | Host `scalar` on PATH; handoff argv (same pattern as `git-lfs` / `gh`). |
| **Effort** | **S** for handoff deepen; **L** for reimplement |
| **Specfold honesty** | Handoff → stays **Claimed-closest / refuse+reason**. In-arks Scalar product = **Non-goal** without SoT reopen. |
| **Recommended next** | Keep refuse; optional docs “install Scalar separately.” |

#### A7. GUI embed (`gitk` / `git-gui` / citool)

| Field | Content |
|-------|---------|
| **Tip status** | **Claimed (FS7 handoff)** — external refuse+reason; not embed Tcl/Tk |
| **Bucket** | **3 FFI / external CLI** (handoff); embed would be **Hybrid 2+3** (Tk/webview) and **Non-goal** |
| **Why** | Tcl/Tk GUIs are separate processes historically; embedding pulls GUI stacks and product scope. |
| **Packages / CLI / FFI** | Spawn `gitk` / `git-gui` / `git citool` when present; else refuse. Optional future: `tauri`/`eframe` — still a new product. |
| **Effort** | Handoff already Done (**S** maintain). Embed **L** + Non-goal. |
| **Specfold honesty** | Handoff **Claimed**. Embedded GUI **Non-goal / Impossible** under FS DoD without product-intent reopen. |
| **Recommended next** | Hold. |

---

### B — Ops / policy / host / crypto / storage

#### B1. Forge host UX (Checks / PR / Issues / merge-queue)

| Field | Content |
|-------|---------|
| **Tip status** | Claimed-closest thin-wrap; full UX **Non-goal** / Impossible under FS DoD |
| **Bucket** | **3 FFI / external CLI** (dominant); optional **2** (`octocrab`/`octorust`) for deepen |
| **Why** | Forge host ≠ revision layer. `gh` already wraps Checks/PR/api. |
| **Packages / CLI / FFI** | Host **`gh`** / `$ARKS_GH` (shipped); optional `octocrab`. |
| **Effort** | Recipe deepen **S**; full UX **L** + SoT reopen |
| **Specfold honesty** | Thin-wrap deepen stays **Claimed-closest**. Full forge product Claimed requires primary SoT reopen — treat as **Non-goal**. |
| **Recommended next** | Hold; optional more `arks forge recipe` coverage only. |

#### B2. Host-env full sync / hydration / LSM bypass

| Field | Content |
|-------|---------|
| **Tip status** | Detect + opt-in refuse / LSM detect **Claimed**; full sync / OneDrive hydration / LSM bypass **Impossible** |
| **Bucket** | Advisory deepen: **Hybrid 2+3**; full claim: **Hold / Impossible** |
| **Why** | Multi-device sync races and CfAPI hydration are host physics; crates cannot warranty safety. |
| **Packages / CLI / FFI** | `windows` Cloud Filter APIs; DriveFS/Dropbox/Syncthing/Nextcloud markers (shipped); `selinux` / `getenforce` / `aa-status`; Docker AppArmor fixture. |
| **Effort** | Further advisory **S–M**; full claim **Impossible** |
| **Specfold honesty** | Detect/refuse already Claimed. **Never** market “cloud-sync safe” / “SELinux fixed.” |
| **Recommended next** | Hold full claim; optional OneDrive-host detect evidence only. |

#### B3. Clean-room Git pin (2.39.5)

| Field | Content |
|-------|---------|
| **Tip status** | Process-closed (FW-PIN / R6) |
| **Bucket** | **Hybrid 1+3** — pin bump is process + image rebuild + G14-class re-prove (not a feature crate) |
| **Why** | Oracle Git lives in Docker image lock; advance is hygiene SoT, not product feature. |
| **Packages / CLI / FFI** | Distro/`git` packages in image; `verify/images.lock.json`; Docker build. |
| **Effort** | **M** (rebuild + full matrix) when deliberate |
| **Specfold honesty** | Pin bump → re-**Claimed** gates after green matrix; until then process-closed is correct. |
| **Recommended next** | Do not bump opportunistically; follow [[sot:root/clean-room-git-pin-advance]] when forced. |

#### B4. Pf5 ALIAS PATH / `exec` (≤0.5×)

| Field | Content |
|-------|---------|
| **Tip status** | ALIAS **0.5546** `aggressive_na`; REVERT Claimed; **no stampede** |
| **Bucket** | **1 Self-implement** (product PATH/`exec` dispatch); harness tweaks are secondary self |
| **Why** | Process-tax is argv/PATH layering vs direct `arks`. Seed amortization exhausted/hurt (U1). No crate removes process spawn tax. |
| **Packages / CLI / FFI** | None. Sole G19 runner only **after** justified product change. |
| **Effort** | **M** for PATH redesign ADR + code; then **one** sole G19 |
| **Specfold honesty** | After one remasure: ≤0.5 → Claimed **or** keep `aggressive_na`. Lucky loops = fraud. In-process multi-command amortize remains **Non-goal** without ADR. |
| **Recommended next** | **Hold** tip honesty. Next eng only if product wants alias ≤0.5: design PATH/`exec` cut → **one** sole G19 — **no thrash**. |

#### B5. Live G8 latency

| Field | Content |
|-------|---------|
| **Tip status** | **Blocked** — secrets=0; harness Claimed; local-forge ≠ live PASS |
| **Bucket** | **3 FFI / external CLI / ops** (Environment secrets + Actions/`gh` + HTTPS peer) |
| **Why** | Measurement substrate is operator PAT + URL; not crates. |
| **Packages / CLI / FFI** | GitHub Environment `g008-live-forge-smoke-live` secrets `ARKS_G8_TOKEN` + `ARKS_G8_REPO_URL`; `gh workflow run g008-live-forge-smoke-forge-scale.yml`; optional compose `g008-live-forge-smoke` / `g008-live-scale`. |
| **Effort** | **Ops** (minutes once secrets exist) + evidence attach |
| **Specfold honesty** | Live receipt → latency row Done. SKIP / dry-run / `claimed-wire` ≠ live PASS. |
| **Recommended next** | Owner fills secrets → one live dispatch → evidence. Highest ops ROI. |

#### B6. L9 crates.io / docs.rs

| Field | Content |
|-------|---------|
| **Tip status** | **Blocked** owner policy |
| **Bucket** | **3 FFI / external CLI / ops** (`cargo publish`, crates.io OIDC, Environment `crates`) |
| **Why** | Publish is owner SoT + trusted publisher — not development Open. |
| **Packages / CLI / FFI** | crates.io OIDC; `cargo publish`; docs.rs follows; reopen pack [[research:root/20261111003444]]. |
| **Effort** | **Ops** after owner reopen |
| **Specfold honesty** | Dry-run ≠ L9 Done. Agent MUST NOT publish. |
| **Recommended next** | Wait for owner SoT; no agent action. |

#### B7. SHA-1 collision **immunity**

| Field | Content |
|-------|---------|
| **Tip status** | Detect+refuse **Claimed**; immunity **Impossible** / Non-goal |
| **Bucket** | **Hold / Impossible** (detect path was **2** — already shipped via `sha1collisiondetection`) |
| **Why** | Absolute immunity is a math claim no library can make true. |
| **Packages / CLI / FFI** | `sha1collisiondetection` already integrated; SHAttered fixtures for regress only. |
| **Effort** | **—** for immunity; optional detect coverage **S** |
| **Specfold honesty** | Keep “detect+refuse”; **never** market “immune.” |
| **Recommended next** | Docs only. |

#### B8. Cloud-sync / hydration / LSM (full) — see B2

Covered under B2. Duplicate name retained for user inventory completeness.

#### B9. Embedded OAuth browser UX

| Field | Content |
|-------|---------|
| **Tip status** | External `credential.helper` honored (**Claimed**); embedded browser OAuth **Impossible** / Refuse |
| **Bucket** | **Hybrid 2+3** if AUTH SoT reopens; else **Hold** |
| **Why** | First-party browser OAuth = forge auth product + App registration. |
| **Packages / CLI / FFI** | `oauth2` + `open`/`webbrowser`; device-flow crates; Git Credential Manager as **external** helper (preferred). |
| **Effort** | Helper-docs **S**; first-party flow **L** + legal |
| **Specfold honesty** | Helper honor stays Claimed. Embedded UX Claimed only after AUTH SoT reopen. |
| **Recommended next** | Document GCM / helper OAuth; no first-party browser. |

#### B10. CELIR ObjectStore

| Field | Content |
|-------|---------|
| **Tip status** | **Refuse-closed (terminal)** — FS `.git` sole object store; index-only `celir-index` Claimed (M24) |
| **Bucket** | **Hold / Impossible** as product path (reverse ADR forbidden) |
| **Why** | Explicit ObjectStore Refuse ([[adr:root/20261109204916]], storage-celir-boundary). Index dual-path already shipped. |
| **Packages / CLI / FFI** | N/A — do not reintroduce ObjectStore via crates. |
| **Effort** | **—** |
| **Specfold honesty** | ObjectStore remains Refuse. Index-only stays opt-in Claimed. |
| **Recommended next** | Do not reopen. |

#### B11. “Faster than Git” (blanket marketing)

| Field | Content |
|-------|---------|
| **Tip status** | Public-surface **Closed** — no blanket slogan; cite receipts only |
| **Bucket** | **Hold** (policy); engineering is **1** only for journey-local wins with evidence |
| **Why** | Marketing claim ≠ feature gap. Comparative ratios are evidence-bound (M17/Pf5). |
| **Packages / CLI / FFI** | None unlock a blanket claim. |
| **Effort** | **—** |
| **Specfold honesty** | Keep receipt-adjacent language in `docs/performance.md`. Blanket “faster than Git” stays **forbidden**. |
| **Recommended next** | No work item. |

#### B12. Multi-writer warranty

| Field | Content |
|-------|---------|
| **Tip status** | Lease L3 / shared-index / catalog Claimed with receipts; **not** a warranty all races solved (`docs/limits.md`) |
| **Bucket** | **Hold** for warranty language; further races = **1 Self-implement** catalog promoteables only |
| **Why** | Catalog Deferred=0 terminals dispositions; marketing “all races fixed” is dishonest. Multi-writer correctness product path is `arks-lease`, already Claimed shape. |
| **Packages / CLI / FFI** | None turn Claimed sets into a universal warranty. |
| **Effort** | **—** for warranty; optional new scenario IDs **M** each |
| **Specfold honesty** | Keep “Claimed set + receipts” wording. Warranty = **Impossible** as absolute claim. |
| **Recommended next** | No warranty work; only SoT-amended new scenario IDs if needed. |

---

## Summary matrix

| Gap | Bucket | Claimed upgrade possible? | Effort | Priority class |
|-----|--------|---------------------------|--------|----------------|
| Exotic filters | **1** | Yes (per family, SoT+) | M–L | Low–med feature |
| Dumb HTTP force-with-lease | **1** | Yes or keep smart-only | M | Low |
| LFS tus/locks/S3 | **2+3** | Locks slice yes; tus/S3 Partial/handoff | M–L | Med if LFS product |
| Server bundle-uri advertise | **1** | Yes | M | Med |
| Mixed sha1/sha256 remotes | **1** / Hold | Refuse honesty only | L | None |
| `scalar` | **3** / Hold | Handoff only | S | None |
| GUI embed | **3** / Hold | Handoff Claimed; embed Non-goal | — | None |
| Forge host UX | **3** (+opt **2**) | Closest only | S | Low optional |
| Host-env full sync/hydration/LSM | Hold (**2+3** advisory) | Full **Impossible** | — | None |
| Git pin | **1+3** process | After matrix | M | Process only |
| Pf5 ALIAS PATH/`exec` | **1** | Maybe ≤0.5 after one G19 | M | Hold unless product |
| Live G8 | **3** Ops | Yes with secrets+receipt | Ops | **Highest ops** |
| L9 | **3** Ops | Yes after owner | Ops | Owner-only |
| SHA-1 immunity | Impossible | **No** | — | None |
| Embedded OAuth | Hold (**2+3**) | After AUTH SoT only | L | None |
| CELIR ObjectStore | Refuse | **No** | — | None |
| “Faster than Git” | Hold | **No** (blanket) | — | None |
| Multi-writer warranty | Hold | **No** (absolute) | — | None |

---

## Ranked “best ROI with deps/FFI acceptance”

Accepting crates, host CLIs, OS APIs, and operator secrets — **without** inventing PASS / G19 thrash / L9 publish:

| Rank | Gap | Why ROI | Bucket | Honest outcome if pursued |
|-----:|-----|---------|--------|---------------------------|
| 1 | **Live G8 latency** | Unblocks PARTIAL→stronger ops Done with zero product redesign | **3** Ops | Live latency Claimed **only** with secrets+receipt |
| 2 | **L9 publish** | Public crate/docs surface; blocked solely on owner | **3** Ops | L9 Done after reopen pack + trusted publish |
| 3 | **Server bundle-uri advertise** | Completes NET-006 server side; pure self; clear evidence shape | **1** | Server advertise Claimed |
| 4 | **LFS locks** (not full S3/tus) | `git-lfs-api` / hand-rolled locks endpoints; high user value vs tus | **2** (+ serve-http **1**) | Locks subset Claimed; tus/S3 still handoff/Partial |
| 5 | **Forge recipe / thin API deepen** | Cheap Claimed-closest; uses `gh` / optional `octocrab` | **3** / **2** | Still not full forge UX |
| 6 | **Exotic filters (`object:type` → `combine:`)** | Self-implement; expands R3 without deps | **1** | Per-family Claimed |
| 7 | **Pf5 ALIAS PATH/`exec`** | Only remaining ≤0.5 journey; **one** G19 after product cut | **1** | Claimed or re-NA — **no thrash** |
| 8 | **Host-env advisory deepen** | Deps/FFI already largely Claimed; diminishing returns | **2+3** | Advisory only; full still Impossible |
| — | Immunity / ObjectStore / blanket faster / multi-writer warranty / mixed-hash / GUI embed / scalar product | Negative or zero ROI | Hold | Keep Impossible / Refuse / Non-goal |

**Explicit non-ROI (do not schedule):** SHA-1 immunity, CELIR ObjectStore reverse, blanket “faster than Git,” multi-writer warranty, embedded OAuth (without AUTH SoT), GUI/`scalar` product embed, inventing live G8 without secrets, G19 stampede for ALIAS.

---

## Refusals honored

- No invent PASS for secrets-absent live G8.  
- No unauthorized L9 / `publish` flip.  
- No G19 remasure in this research task.  
- No Claimed full FS-007 / WIN-003 hydration / FS-014 bypass / crypto immunity / forge Checks product / ObjectStore.

## Cross-links

| Artifact | Role |
|----------|-------|
| [[research:root/20261111012840]] | Still-blocked external-deps Plan A/B |
| [[research:root/20261110222000]] | FFI / OS deep dive (sha1cd, host-env, forge) |
| [[research:root/20261111001816]] | Partial-completion unblock sequence |
| [[sot:root/program-roadmap]] v0.15.4 | Tip Done/Partial/Blocked/Non-goal |
| [[sot:root/production-residual-register]] | §F terminal + reopen paths |
| `docs/completion.md` / `docs/git-vs-arks.md` / `docs/limits.md` | Public PARTIAL honesty |
