---
id: research:root/20261110222000
title: Remaining gaps — FFI / external packages / OS APIs unblock research
status: active
created: 2026-11-10T22:20:00Z
updated: 2026-11-10T22:20:00Z
tags:
  [
    remaining-gaps,
    ffi,
    external,
    g008-live-forge-smoke-live,
    l9,
    obj-004,
    sha1collisiondetection,
    fw-forge-b,
    oauth,
    fs-007,
    fs-014,
    win-003,
    google-drive,
    drivefs,
    host-advisory,
  ]
rel:
  depends_on:
    - research:root/20261110211804
    - research:root/20261110212414
    - sot:root/production-residual-register
    - evidence:root/20261110210200
    - evidence:root/20261110221919
    - adr:root/20261110220000
    - adr:root/20261110221000
---

# Remaining gaps — FFI / external / OS unblock research

## Question

After Full Symmetry **FS0–FS_final Done** at tip ≥ `04f40bc`, which remaining inventory items are still **implementable in-tree**, which stay **secrets/owner-blocked**, and for each prior **Impossible**, what concrete **external package / FFI / OS API / subprocess** paths exist — with Specfold honesty (Claimed vs thin-wrap vs advisory)?

## Method

1. Re-probe Environment `g008-live-forge-smoke-live` secret *names* via `gh` API + process env.
2. Ship structural readiness + optional **local-forge wire** substitute (≠ GitHub live PASS).
3. Read-only inspect this host’s **G:** Google Drive for desktop substrate; run a bounded throwaway `.git` probe; record advisory evidence.
4. Deep-research each Impossible item against crates.io / system libs / stock tools (`gh`, `aa-status`, Cloud Filter APIs, `sha1collisiondetection`).
5. Rank next-sprint recommendations. **No invent Claimed/PASS. No L9 publish. No OWNER.md.**

## Tip honesty baseline (this session)

| Item | Value |
|------|-------|
| Tip | `04f40bc` — *rustfmt LFS network and sha256 porcelain surfaces.* |
| Environment `g008-live-forge-smoke-live` secret count | **0** (re-probed 2026-11-10T22:17Z / 22:19Z) |
| Process `ARKS_G8_TOKEN` / `ARKS_G8_REPO_URL` / `GITHUB_TOKEN` | **Absent** |
| Live G8 remasure | **Still secrets-blocked** |
| Local-forge wire harness | **Shipped + Docker green** — compose `g008-local-forge` → `claimed-wire` ([[evidence:root/20261110222100]]); ≠ GitHub live PASS |
| G8 live script fix | `run-g008-live-forge-smoke.sh` corrected for arks push surface (`branch` + named push; no `-c`/`-u`/src:dst) |
| G: DriveFS advisory probe | [[evidence:root/20261110221919]] — **advisory only** |

---

## Part A — Implement possible (session results)

### Live G8 (`ARKS_G8_TOKEN` + `ARKS_G8_REPO_URL`)

| Layer | Result |
|-------|--------|
| Secrets present? | **No** (`total_count=0`) |
| Live remasure | **Not run** — would invent PASS if forced |
| Structural readiness | **OK** (`scripts/forge-scale-readiness.sh`) |
| Local substitute | **Shipped** — loopback `arks serve-http` clone/push/fetch → `arks-g008-local-forge-wire/v1` with `github_live_pass=false` |

**Exact operator steps (unchanged honesty):**

1. Create throwaway peer repo; mint scoped PAT.
2. Environment `g008-live-forge-smoke-live` secrets: `ARKS_G8_TOKEN`, `ARKS_G8_REPO_URL`.
3. Confirm: `gh api repos/modootoday/arks/environments/g008-live-forge-smoke-live/secrets` → `total_count ≥ 2`.
4. `gh workflow run g008-live-forge-smoke-forge-scale.yml -f dry_run=false` **or** compose `g008-live-forge-smoke` + `g008-live-scale` with env exports.
5. Attach real receipts under `.spec/evidence/` — SKIP ≠ PASS.

Until secrets exist: use compose `g008-local-forge` for **wire** proof only.

### Other in-tree residuals shipped this session

- Honesty notes: `FS-007` / `FS-014` / `WIN-003` in `arks lock` catalog notes (`crates/arks/src/cmd/lock.rs`).
- Docs: `docs/forge-scale.md` local-forge section; readiness script checks `g008-local-forge`.

### L9 crates.io / docs.rs

**No flip.** Research unblock path only (below). Owner SoT required before `publish = true` or trusted-publisher live job.

---

## Part B — Deep research per Impossible item

Verdict vocabulary:

- **Implementable via ext/FFI (recommend path)** — concrete crate/API; Claimed slice possible after SoT amend + evidence.
- **Partial** — detect/honor/wrap possible; full product Claimed still blocked by scope or host.
- **Still Impossible** — hard reason that no external package removes.

### 1. OBJ-004 — SHA-1 collision immunity (crypto)

| Vector | Detail |
|--------|--------|
| Prior closest | `arks fsck` OidMismatch refuse + sha256 porcelain Claimed |
| External | crates.io **`sha1collisiondetection`** (MIT; Sequoia c2rust port of Marc Stevens / git.git submodule); C lib `libsha1detectcoll` / `sha1dcsum` |
| Git precedent | git.git uses collision-detecting SHA-1 for object hashing |
| Effort | Medium — swap/wrap hasher behind object OID path; fixture with SHAttered PDFs; gate honesty |
| License / security | MIT; well-reviewed counter-cryptanalysis; ~2× SHA-1 CPU |
| Specfold honesty | **Detection + safe-hash refuse** = Claimable slice; **absolute cryptographic immunity** remains Impossible as a math claim |

**Verdict: Implementable via ext/FFI (recommend path)** for *collision-attack detection / refuse*  
**Still Impossible** for *immunity* wording.

Recommended path: optional feature `sha1cd` → `sha1collisiondetection::Sha1CD` on ingest/`hash-object`/`index-pack`/`fsck`; refuse or safe-hash on `found_collision`; evidence with known colliding pair; residual text must say **detect+refuse**, not immune.

### 2. FW-FORGE-B — Checks / PR / Issues UX (not-git)

| Vector | Detail |
|--------|--------|
| External | **`octocrab`** / `octorust` Checks+Issues+PR APIs; subprocess **`gh pr` / `gh api`** |
| Effort | Low–medium for a thin CLI helper; high for product UX parity with GitHub UI |
| Specfold | arks is revision layer (scope B). Shipping forge UX as first-party **redefines product** |

**Verdict: Partial** — implementable as **optional operator helper / docs “use `gh`”** (thin-wrap).  
**Still Impossible** as *in-arks forge Checks/PR product Claimed* without primary SoT reopen that arks becomes a forge client product.

Recommend: keep Impossible; optionally document `gh` recipes; do not Claim under CLI-parity.

### 3. Forge OAuth browser UX (not-git)

| Vector | Detail |
|--------|--------|
| External | Device-flow crates (`github-device-flow`, `ghtkn`); `oauth2` + `open`/`webbrowser`; Git Credential Manager as **external** helper |
| Existing Claimed | G33 credential helper / askpass / OS wallets; external helper MAY speak OAuth |
| Specfold | Embedded browser OAuth UX = forge auth product (Refuse today) |

**Verdict: Partial** — **honor external helper** already Claimed; bundling browser OAuth UX remains product/legal boundary.  
**Still Impossible** for *first-party in-arks browser OAuth Claimed* without AUTH SoT reopen + App client registration.

Recommend: document GCM/`credential.helper` OAuth; no first-party browser flow unless owner opens AUTH.

### 4. FS-007 — cloud-sync on `.git` (host-env)

| Vector | Detail |
|--------|--------|
| Detect | Path markers: `.dropbox`, `.syncthing`, OneDrive root via Known Folder; DriveFS drive letter + `GoogleDriveFS` process / `HKCU\Software\Google\DriveFS`; volume label `Google Drive` |
| OneDrive | `windows` crate Cloud Filter (`CfGetPlaceholderStateFromAttributeTag`); `FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS` |
| DriveFS | Virtual Dokan/custom FS (FAT32 letter); **not** CfAPI placeholders — different substrate |
| Cannot package away | Multi-device races, server conflict copies (`file (1).txt`), remote lock steal |

**Verdict: Partial** — **detect + warn/refuse multi-writer Claimed** is implementable;  
**Still Impossible** to *implement cloud sync correctness* or Claim full FS-007 hazard closure.

See **Host Google Drive (G:)** section below for empirical substrate.

### 5. FS-014 — SELinux / AppArmor deny lock rename (host-env)

| Vector | Detail |
|--------|--------|
| Detect | `/sys/fs/selinux/enforce`; `/sys/module/apparmor/parameters/enabled`; subprocess `aa-status`, `getenforce` |
| FFI | crates.io **`selinux`** (libselinux bindings); AppArmor mostly sysfs + `libapparmor` |
| Induce | Needs confined profile / policy — Docker can approximate with custom AppArmor profile; not portable to this Windows host |
| Closest product | Surface `ErrorKind::PermissionDenied` with hint naming LSM when detectable |

**Verdict: Partial** — detect + errno honesty + optional Docker policy fixture.  
**Still Impossible** to *implement / bypass host LSM* inside arks.

### 6. WIN-003 — OneDrive Files On-Demand (host-env)

| Vector | Detail |
|--------|--------|
| FFI | `windows` / `cloud-filter` crates; `CfGetPlaceholderStateFromAttributeTag`; attribute `RECALL_ON_DATA_ACCESS` |
| Effort | Low–medium for detect+honest errno on hydrate failure |
| This host | **G: is DriveFS**, not OneDrive CfAPI — does **not** empirically exercise WIN-003 placeholders |

**Verdict: Implementable via ext/FFI (recommend path)** for *detect + honesty* slice.  
**Still Impossible** to *implement OneDrive hydration* or eliminate host placeholder semantics.

### 7. L9 — crates.io / docs.rs (owner) — research only

| Vector | Detail |
|--------|--------|
| Unblock | Owner SoT reopen → flip selected `publish`; crates.io OIDC trusted publisher + Environment `crates`; tag `v*`; docs.rs follows publish |
| Subprocess | `cargo publish --dry-run`; `gh` release; never agent-unauthorized live publish |
| This session | **No flip** |

**Verdict: Partial (ops path known)** / **Still closed** until owner speaks — not a Specfold development Open.

---

## Host Google Drive (G:) as cloud verification substrate

### Observed host facts (read-only + bounded probe)

| Observation | Value |
|-------------|-------|
| `Get-PSDrive G` Description | `Google Drive` |
| Volume | `VolumeName=Google Drive`, `FileSystem=FAT32`, `DriveType=3` (~1 TB virtual size) |
| Process | `GoogleDriveFS.exe` (versions **132.0.0.0** and **133.0.2.0** co-resident) |
| Driver | `C:\Program Files\Google\Drive File Stream\Drivers\*\googledrivefs*.sys` |
| Registry | `HKCU\SOFTWARE\Google\DriveFS\Share` (`BasePath`, `SyncTargets`, …) |
| `G:\` reparse | **Error 4390** — not a reparse point |
| Sample My Drive files | Normal attrs; **no** CfAPI reparse; `.gsheet` stubs ~193 B |
| `mirror_item` rows | **0** → **streaming-likely** (virtual letter, not mirrored local folder) |
| Bounded probe | [[evidence:root/20261110221919]] — `git init/add/commit`, lock create/rename OK; cleaned |

### Does G: enable meaningful FS-007 verification?

**Yes — as Useful advisory / Partial substrate. Not sufficient for Claimed FS-007.**

| Verifiable on G: (single host) | Not verifiable here |
|-------------------------------|---------------------|
| Repo create under synced/streamed tree | Multi-device concurrent writers |
| Lockfile create / rename errno behavior on DriveFS | Server-side conflict duplicate files |
| Latency / flaky I/O under virtual FS | Offline↔online hydration races (DriveFS cache) without controlled network cut |
| Detect DriveFS root (letter + process + volume label) | OneDrive **CfAPI** placeholder semantics (WIN-003) |
| Advisory “don’t put `.git` on cloud sync” demos | SELinux/AppArmor (FS-014) |
| Compare stock git vs arks on same path (future harness) | Claiming cloud-sync-safe multi-writer |

**Why not Claimed:** FS-007’s hazard model is **cross-device sync conflict / corruption**, which a single Windows session cannot induce honestly. DriveFS streaming also differs from OneDrive Files On-Demand — passing G: smoke must not be marketed as WIN-003 Claimed.

### Verdict

**Useful advisory / Partial** — excellent host for:

1. Sync-root **detection** fixtures (DriveFS).
2. Single-host **refuse/warn** UX when `.git` lives under `G:\`.
3. Performance/flake advisory gates labeled `host-advisory`.

**Not sufficient for Claimed FS-007.**

### Recommended harness design

1. **Detect module** (Windows): DriveFS (`GoogleDriveFS` + volume label / fixed letter from registry); OneDrive (`windows` Cloud Filter APIs); Dropbox/Syncthing markers.
2. **Advisory gate** `verify/scripts/run-fs007-host-advisory.sh` — opt-in env `ARKS_FS007_PROBE_ROOT` pointing at a user-accepted sandbox under `G:\…`; stock git + arks smoke; receipt schema with `claimed=false`.
3. **CLI honesty**: on detect, print FS-007 note (already seeded in `arks lock` notes); optional hard refuse behind config flag (SoT amend before Claimed).
4. **Separate WIN-003 lane**: require a real OneDrive placeholder tree; do not reuse DriveFS evidence.
5. **Never** promote advisory receipts to residual Claimed without multi-device protocol (out of band / two machines).

---

## Ranked recommendation — next implementation sprint

| Rank | Item | Action | Effort | Honesty class |
|------|------|--------|--------|---------------|
| 1 | OBJ-004 detect | Integrate `sha1collisiondetection`; refuse/safe-hash; SHAttered fixture | M | Claimable **detect+refuse** (not immunity) |
| 2 | WIN-003 detect | `windows` CfAPI / `RECALL_ON_DATA_ACCESS` honesty | S–M | Claimable detect slice; need OneDrive host |
| 3 | FS-007 detect | DriveFS + sync-root detectors; advisory gate using G: sandbox | S–M | Advisory → optional refuse; **not** full FS-007 Claimed |
| 4 | Live G8 | Owner fills `g008-live-forge-smoke-live` secrets; remasure + evidence | Ops | Live PASS only with receipts |
| 5 | FS-014 | errno hints + optional AppArmor Docker fixture | S | Partial honesty |
| 6 | Forge OAuth / FW-FORGE-B | Docs + `gh` / external helper only | S | Keep Impossible as product |
| 7 | L9 | Wait for owner SoT | — | Closed |

---

## Session deliverables

| Artifact | Path |
|----------|------|
| Research (this note) | `.spec/research/20261110222000-remaining-gaps-ffi-external-unblock.research.md` |
| G: advisory evidence | `.spec/evidence/20261110221919-fs007-gdrive-host-advisory/` |
| Local-forge script | `verify/scripts/run-g008-local-forge.sh` |
| Local-forge evidence | `.spec/evidence/20261110222100-g008-local-forge-wire-advisory/` |
| Compose service | `g008-local-forge` in `verify/docker-compose.yml` |
| Readiness / docs | `scripts/forge-scale-readiness.sh`, `docs/forge-scale.md` |
| Catalog notes | `FS-007` / `FS-014` / `WIN-003` in `crates/arks/src/cmd/lock.rs` |

## Non-goals / refusals honored

- No invented GitHub live PASS / scale SLO.
- No L9 `publish` flip / crates.io upload.
- No OWNER.md.
- No force-push.
- G: probe read-only except bounded throwaway dir (cleaned).
