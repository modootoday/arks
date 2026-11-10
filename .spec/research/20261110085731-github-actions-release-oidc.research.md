---
id: research:root/20261110085731
title: GitHub Actions CI, Cross-OS, perf honesty, and OIDC multi-registry release
status: active
created: 2026-11-10T08:57:31Z
updated: 2026-11-10T00:35:00Z
tags:
  [
    github-actions,
    ci,
    cross-os,
    oidc,
    trusted-publishing,
    npm,
    pypi,
    crates-io,
    rubygems,
    winget,
    apt,
    homebrew,
    chocolatey,
    release,
    perf,
    perf-ci,
    g16,
    g19,
    research,
    deferred-signing-channels,
    runner-annotations,
  ]
rel:
  depends_on:
    - sot:root/product-verification-clean-room
    - sot:root/clean-room-journey-compare
    - sot:root/interop-test-matrix
    - sot:root/public-surface
    - sot:root/program-roadmap
    - research:root/20261108073650
    - research:root/20261109125438
  related_plan:
    - plan:root/20261110085731
---

# GitHub Actions CI, Cross-OS, perf honesty, and OIDC multi-registry release

Design research for **arks** (Rust CLI) continuous integration expansion, Cross-OS native testing, honest performance measurement venues, and **OIDC / trusted-publishing** release to language registries. Normative “verified” remains Docker clean-room ([[sot:root/product-verification-clean-room]]); GHA is CI/release machinery, not a substitute oracle.

**Implementation status (2026-11-10):** Workflows landed — `.github/workflows/ci.yml`, `.github/workflows/release.yml`. Publish jobs use Environments `crates` / `npm` / `pypi` / `rubygems` (required reviewer `myso-kr`, tag policy `v*`). Runner pins avoid deprecated Annotation labels — see [Runner-image Annotations](#runner-image-annotations-2026-11). Signing channels remain OUT. Registry publish steps fail-closed / skip when packaging or OIDC trust is not ready (no long-lived tokens).

**Scope honesty (2026-11-10):** Channels that require **certificate signing / code signing / publisher identity / notarization / paid publisher certs / long-lived package-repo GPG** are **OUT of current release-channel scope** (Deferred). Near-term publish surface is OIDC registries + GitHub Releases with **unsigned or checksum-only** artifacts. See [Current release-channel scope](#current-release-channel-scope).

## Question

How should arks structure GitHub Actions for (1) Cross-OS functional CI, (2) journey/G19-class timing honesty vs clean-room Docker, and (3) OIDC publish to npm / PyPI / crates.io / RubyGems without long-lived tokens — while shipping **one Rust binary** through multi-language wrappers? (OS package catalogs such as apt / winget remain survey-only; they are **excluded** from near-term scope for signing constraints.)

## Method

1. Read primary trusted-publishing / OIDC docs for npm, PyPI, crates.io, RubyGems, and GitHub OIDC.
2. Survey apt / winget / Homebrew / Chocolatey / Authenticode / Apple notarization **only** to classify signing constraints; mark constrained channels OUT / Deferred (do not treat as in-scope Phase 3–4 deliverables).
3. Cross-walk Rust Cross-OS matrix / MSRV / cache / artifact patterns against arks clean-room SoT (G16/G19 venue rules).
4. Record wrapper patterns: npm platform packages / napi-class optionalDependencies, maturin `bindings = "bin"` wheels, Ruby native/platform gems.
5. Produce phased recommendation + explicit **What NOT to do** + channel KEEP/EXCLUDE table.

## Pillars (first-class)

| # | Pillar | Role |
|---|--------|------|
| A | **Cross-OS CI** | Native `cargo test` / CLI smoke on Windows / macOS / Linux runners; MSRV; artifacts |
| B | **Perf CI + measurement honesty** | G19-class timing only in clean-room Docker; GHA = advisory smoke/trends; no Verified `perf.yml` required |
| C | **CI expansion** | Lint/fmt/clippy, matrix, cache, cancel-in-progress, release job graph |
| D | **OIDC multi-registry release** | crates.io + npm + PyPI + RubyGems trusted publishing (**in scope**) |
| E | **OS package channels (survey only)** | apt / winget / Chocolatey / Authenticode / Apple notarization — **OUT / Deferred** (signing constraints); Homebrew tap optional later without claiming notarization |

## Current release-channel scope

Owner direction: among surveyed release channels, **exclude** those with certificate-signing (or similar hard external) constraints from **current** scope. Feasibility notes below remain for later re-open; they MUST NOT be read as Phase commitments.

| Channel | Scope | Reason |
|---------|-------|--------|
| **crates.io** (OIDC trusted publishing) | **KEEP** | Token/OIDC; no code-signing cert |
| **npm** (OIDC trusted publishing + provenance) | **KEEP** | OIDC / Sigstore-class provenance ≠ paid Authenticode/Apple cert |
| **PyPI** (OIDC Trusted Publishers) | **KEEP** | Token/OIDC; no code-signing cert |
| **RubyGems** (OIDC trusted publishing) | **KEEP** | Token/OIDC; optional Phase 2 after npm/PyPI/crates |
| **GitHub Releases** (binaries + SHA256SUMS) | **KEEP** | Checksum-only / unsigned artifacts OK; `contents: write` |
| **Cross-OS GHA build/test matrix** | **KEEP** | No signing requirement |
| **Unsigned / checksum-only `.deb` / `.msi` / tarball as Release assets** | **KEEP (asset only)** | Shipping bytes on a Release is not claiming an apt/winget **channel** |
| **winget** (`winget-pkgs` / Publisher Identity / Authenticode practical bar) | **EXCLUDE / Deferred** | Catalog + SmartScreen/Authenticode / Microsoft publisher-identity friction; not OIDC |
| **apt** (PPA / self-hosted signed repo) | **EXCLUDE / Deferred** | Inherent **GPG package + repo signing** + hosting/ops; not GitHub OIDC |
| **Chocolatey** | **EXCLUDE / Deferred** | API-key path still overlaps Windows discovery; trusted/package-signing expectations; avoid dual Windows channel |
| **Windows Authenticode-signed installers** | **EXCLUDE / Deferred** | Paid/EV code-signing cert + key ceremony |
| **Apple Developer ID + notarization** (signed/notarized macOS installers) | **EXCLUDE / Deferred** | Apple Developer Program + notarization pipeline |
| **Homebrew** (user tap downloading checksummed Release assets) | **Deferred (non-cert)** | Tap alone is not a paid-cert channel; still **out of near-term phases** until demand; do **not** imply Apple notarization |

**Ambiguity (honest):** npm/PyPI/crates **attestation / provenance** (OIDC → Sigstore-style) is **kept** and is **not** the same class of constraint as Authenticode, Apple notarization, or apt **repo GPG** keys. Optional maintainer GPG for crates.io crate signing (if ever claimed) is a separate decision and is **not** required for OIDC `cargo publish`.

---

## A. Cross-OS testing (GitHub Actions)

### Intent

arks is a **native Rust CLI**. Functional correctness on Windows path/symlink/line-ending semantics, macOS ARM runners, and Linux glibc MUST be exercised on **native GHA runners**. That is **orthogonal** to product-verified Docker clean-room ([[sot:root/product-verification-clean-room]]): Cross-OS CI proves “builds and behaves on host OS”; clean-room proves “interop/journey/verified” against pinned in-image stock Git.

### Recommended matrix (Phase 1 CI)

| Dimension | Values | Notes |
|-----------|--------|-------|
| `os` | `ubuntu-24.04`, `windows-2025`, `macos-15` | **Pinned** (not `*-latest`); `fail-fast: false`. See [Runner-image Annotations](#runner-image-annotations-2026-11) |
| Toolchain | `stable` on all three; **MSRV** on `ubuntu-24.04` only | Read MSRV from workspace `rust-version` (`1.85`) |
| Target | Host triple (native) | Do not confuse cross-compile-only jobs with native tests |
| Profile | `debug` tests on PR; optional `release` smoke on main/tag | Release binary build is a separate `dist` job |

Optional later: `macos-15-intel` (or documented Intel label) if x86_64 Darwin remains a supported install target; Linux `aarch64` via native arm runner or cross + qemu (prefer native when available).

### Native binary tests vs Docker-only verify

| Lane | Where | Authority |
|------|-------|-----------|
| Unit / integration / CLI smoke | GHA matrix (native OS) | **Advisory** for product “verified”; **required** for Cross-OS ship claim |
| G0–G37 interop, G16 journey func, G19 timing bars | Docker clean-room (`verify/` compose) | **Authoritative** — same images CI MUST invoke ([[sot:root/product-verification-clean-room]]) |
| Host laptop | Developer machine | Advisory only |

**Rule:** A green Windows/macOS GHA job MUST NOT be labeled product-verified. A green clean-room job MUST NOT be treated as proof of Windows path semantics.

### Caching pitfalls (Cross-OS)

| Pitfall | Mitigation |
|---------|------------|
| Shared cache key across OS/toolchain → corrupt / thrash | Key by `${{ runner.os }}` + toolchain + `Cargo.lock` hash; prefer `Swatinem/rust-cache@v2` with distinct `key` / `prefix-key` |
| PR jobs overwrite main cache | `save-if: ${{ github.ref == 'refs/heads/main' }}` (or equivalent) |
| Cache used for **release** publish builds | Prefer cold or locked release builds; npm docs explicitly discourage package-manager cache on release (`package-manager-cache: false`) |
| Stale `target/` after toolchain bump | Bump `prefix-key` / delete Actions caches on mysterious compile fails |
| Windows C: I/O slowness | Expect longer Windows jobs; do not use Windows wall times as perf oracle |

### Artifacts

1. **CI test:** no need to upload full `target/` — upload test logs / JUnit only if useful.
2. **Dist job (tag/main):** matrix build release binaries per triple → `actions/upload-artifact` → GitHub Release assets (checksums).
3. **Downstream wrappers:** language packages **consume the same Release assets** (or rebuild identical targets in matrix) — one canonical binary set.

### MSRV

- Declare `rust-version` in workspace; CI job `msrv` runs `cargo test` (or `cargo check` + focused tests) on that toolchain on at least Linux.
- Do not claim MSRV from a single `cargo check` on one OS if Windows-only deps exist (none expected for core CLI today — re-check when wrappers land).

### Primary cites (Cross-OS / runners)

| Source | URL | Scope used |
|--------|-----|------------|
| GitHub-hosted runners | https://docs.github.com/en/actions/concepts/runners/github-hosted-runners | Ephemeral VMs; not a controlled perf lab |
| GitHub-hosted runners reference | https://docs.github.com/en/actions/reference/runners/github-hosted-runners | Labels, slim vs standard, larger runners |
| Rust Project Primer — GHA | https://rustprojectprimer.com/ci/github.html | Matrix, `Swatinem/rust-cache`, Windows rationale |
| Microsoft Rust Training — CI chapter | https://microsoft.github.io/RustTraining/engineering-book/ch11-putting-it-all-together-a-production-cic.html | Cache key / `save-if` pitfalls |

### Runner-image Annotations (2026-11)

GitHub Actions job logs show **Annotations** when a `runs-on` label points at a deprecated / browning / migrating image. As of **2026-11-10**, arks pins versioned labels and **avoids** labels that carry active deprecation/brownout notices.

| Label | Status (2026-11-10) | arks disposition |
|-------|---------------------|------------------|
| `ubuntu-22.04` / `ubuntu-22.04-arm` | Deprecation began **2026-09-17**; brownouts Mar–Apr 2027; unsupported **2027-04-17** | **Do not use** |
| `ubuntu-latest` | Migrating **Ubuntu 24.04 → 26.04** (rollout **2026-10-19 … 2026-11-19**) | **Do not use** during migration; pin instead |
| `ubuntu-24.04` | Supported (GA LTS image) | **Pin for CI + release Linux** |
| `ubuntu-26.04` | GA since **2026-09-17**; optional migrate target after validation | Defer until CI green on 24.04 proves need |
| `windows-latest` / `windows-2025` | VS2026 migration completed **2026-06-08 … 2026-06-15** | **Pin `windows-2025`** (not `windows-2022` unless VS2022 required) |
| `macos-14` (+ large/xlarge) | Brownouts Oct 2026; **retired 2026-11-02** | **Do not use** |
| `macos-latest` | Points at **macOS 26** (migration began **2026-06-15**) | Prefer pin over floating latest |
| `macos-15` | Supported arm64 | **Pin for CI + Darwin arm64 dist** |

**Chosen `runs-on` pins for this repo:** `ubuntu-24.04`, `windows-2025`, `macos-15`.

**Note vs sibling `specfold`:** specfold historically built Linux dist on `ubuntu-22.04` for older glibc. That label now carries deprecation Annotations — arks **must not** copy it; accept glibc from 24.04 for unsigned Release tarballs until a deliberate musl/older-glibc strategy is adopted.

#### Primary cites (runner Annotations / migrations)

| Source | URL | Scope used |
|--------|-----|------------|
| Ubuntu 26 GA + `ubuntu-latest` migration | https://github.blog/changelog/2026-09-17-ubuntu-26-generally-available-and-latest-migration/ | 26.04 GA; latest → 26.04 Oct 19–Nov 19 2026; pin `ubuntu-24.04` to stay |
| Upcoming image migrations | https://github.blog/changelog/2026-05-14-github-actions-upcoming-image-migrations/ | `windows-latest`/`windows-2025` → VS2026 (Jun 2026); `macos-latest` → macOS 26 |
| macOS 14 retirement | https://github.blog/changelog/2026-10-01-github-actions-macos-14-runner-image-retirement/ | `macos-14*` brownouts + retirement 2026-11-02; migrate to 15/26 |
| Ubuntu 22 deprecation issue | https://github.com/actions/runner-images/issues/14254 | Deprecation 2026-09-17; unsupported 2027-04-17; brownout calendar; migrate to 24.04/26.04 |
| Hosted runners reference | https://docs.github.com/en/actions/reference/runners/github-hosted-runners | Current label → image mapping table |

---

## B. Performance measurement honesty + perf CI disposition (G16/G19-class)

### Venue law (arks)

- **Authoritative comparative timing** (G16 record + G19 `arks_median ≤ 0.8× git_median` allow-list): Docker clean-room only ([[sot:root/clean-room-journey-compare]], [[sot:root/product-verification-clean-room]]). Compose services: `g16` (`verify/scripts/run-g16.sh`), `g19` (`verify/scripts/run-g19.sh`); matrices `verify/matrix-journey.md`, `verify/matrix-g19.md`.
- **GHA hosted runners** are shared-pool, multi-tenant, OS-image-variable machines. Suitable for **compile/test**, not for “verified faster than Git” or G19 pass/fail ([GitHub-hosted runners](https://docs.github.com/en/actions/concepts/runners/github-hosted-runners)).
- Product-hotspot research beyond M17 lives separately in [[research:root/20261109125438]] — that note is **not** a GHA perf-CI design; this section owns CI venue honesty.

### What GHA MAY do for perf

| Allowed | Forbidden as “verified” |
|---------|-------------------------|
| Compile-time / test-time budgets (timeout detection) | Publishing GHA wall-ms as G19 receipt |
| Coarse smoke: “journey script exits 0” on pinned `ubuntu-24.04` | Claiming ≤0.8× vs stock Git from noisy runner |
| Nightly advisory trend charts labeled `host-advisory` / `gha-advisory` | README marketing “faster than Git” from Actions |
| Trigger clean-room job that runs the **same** `verify/` compose | Divergent “fast path” bench recipes only on GHA |
| Image-build / compose toolchain smoke (current `clean-room-smoke`) | Relabeling image smoke as G16/G19 Verified |

### Perf CI job design (research disposition)

| Lane | Where | Trigger | Evidence label | Current scope |
|------|-------|---------|----------------|---------------|
| **G16 func + G19 timing (authoritative)** | Sole-runner Docker compose `g16` / `g19` | Human / sole-runner lab | `clean-room` / Specfold evidence | **KEEP** as only Verified path; **OUT of GHA as Verified oracle** |
| **Clean-room image smoke** | GHA `clean-room-smoke` in `.github/workflows/ci.yml` | PR + push `main` | advisory (image builds) | **LANDED** (Phase 1) — proves compose toolchain compiles; MUST NOT claim G0–G37 |
| **Optional journey exit-0 smoke** | Future GHA job invoking the **same** `verify/` compose services without promoting wall-ms | `workflow_dispatch` / optional schedule | `gha-advisory` | **Deferred optional** — only if CI wants regression “exits 0”; never a G19 bar |
| **Nightly advisory trend charts** | Future scheduled workflow uploading labeled JSON/plots | `schedule` cron | `host-advisory` / `gha-advisory` | **Deferred optional** — never public “faster than Git”; never Specfold Verified |
| **Dedicated `perf.yml` / Criterion microbench CI** | Separate workflow | — | — | **Not required** for this research exit; Criterion/host benches remain host-advisory if added later |

**Research exit for “perf CI”:** Venue law + KEEP/OUT/Deferred job table above is the landed Specfold answer. Phase 1 does **not** require a separate `perf.yml`. Optional advisory jobs MAY be authored later under this disposition without reopening G19 venue.

### Caching vs timing

- Dependency/`target` caches **distort** wall-clock of builds; never mix cached release builds into comparative timing.
- Journey timing MUST disable or isolate caches that change I/O shape of the measured child processes; clean-room already pins environment.
- GHA `Swatinem/rust-cache` is fine for **compile/test** jobs; it MUST NOT feed authoritative G19 child-process timing.

### What NOT to do (perf)

1. Treat noisy GHA runner ratios as G19 **Verified**.
2. Compare arks on a warm cache job against git cold (or reverse).
3. Use Windows runner disk latency as evidence of product regression.
4. Skip clean-room because “CI already timed it.”
5. Land a `perf.yml` that claims Verified ≤0.8× from hosted runners.
6. Run concurrent G19-class comparative jobs on shared CI in a way that invents a second Verified oracle beside sole-runner Docker.

#### Primary cites (perf CI / venue)

| Source | URL | Scope used |
|--------|-----|------------|
| GitHub-hosted runners | https://docs.github.com/en/actions/concepts/runners/github-hosted-runners | Shared pool; not a controlled perf lab |
| Events that trigger workflows (`schedule`, `workflow_dispatch`) | https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows | Optional advisory cron / manual smoke shapes |
| arks clean-room SoT | [[sot:root/product-verification-clean-room]] | Host/GHA green ≠ product verified |
| arks journey-compare SoT | [[sot:root/clean-room-journey-compare]] | `run.venue` = `clean-room` vs `host-advisory` |
| G19 matrix | `verify/matrix-g19.md` | Authoritative comparative bar + sole Docker `g19` |

---

## C. CI expansion (job graph)

Implemented under `.github/workflows/` (2026-11-10):

```text
PR / push (main) — ci.yml:
  lint (fmt, clippy -D warnings)     [ubuntu-24.04]
  test-matrix (os × stable)          [ubuntu-24.04|windows-2025|macos-15]
  msrv (1.85)                        [ubuntu-24.04]
  clean-room-smoke (image build)     [ubuntu-24.04 + Docker]  ← NOT product-verified
  # No perf.yml: G16/G19 Verified stays sole-runner Docker (research §B)

Tag v* — release.yml:
  dist-binaries (os/arch matrix)     [ungated]
  github-release (draft + SHA256SUMS)[ungated; checksum/unsigned assets]
  publish-crates   (OIDC)  [environment: crates]     # skip if publish=false
  publish-npm      (OIDC)  [environment: npm]        # skip until npm/ tree exists
  publish-pypi     (OIDC)  [environment: pypi]       # skip until PyPI packaging exists
  publish-rubygems (OIDC)  [environment: rubygems]   # skip until gemspec exists
  # OUT / Deferred (signing-constrained): submit-winget, apt-repo, chocolatey,
  # Authenticode MSI/EXE, Apple notarized installers
  # Deferred (non-cert): homebrew-tap if demand
```

**Secrets / OIDC posture**

| Need | Mechanism | Scope |
|------|-----------|-------|
| npm / PyPI / crates.io / RubyGems publish | OIDC `id-token: write` + registry trusted publisher; **no** long-lived publish tokens | **KEEP** |
| GitHub Release upload | `contents: write` (GITHUB_TOKEN) | **KEEP** |
| winget-pkgs PR | GitHub PAT / fine-grained token (`WINGET_CREATE_GITHUB_TOKEN` pattern) — **not** OIDC; plus Authenticode/publisher-identity friction | **EXCLUDE / Deferred** |
| apt repo / PPA | Long-lived **GPG** signing key secret (inherent); Launchpad trusts GPG, not GitHub OIDC | **EXCLUDE / Deferred** |
| Chocolatey | Community API key secret (no OIDC); package-signing / dual-channel cost | **EXCLUDE / Deferred** |
| Authenticode / Apple notarization | Paid code-signing / Developer ID certs + notarization secrets | **EXCLUDE / Deferred** |
| Homebrew core / private tap | Maintainer/Bottle bot path; tap repo permissions (not paid cert by itself) | **Deferred (non-cert)** |

Prefer **per-registry GitHub Environments** `crates` / `npm` / `pypi` / `rubygems` (sibling pattern; live on `modootoday/arks` with required reviewer `myso-kr` + tag policy `v*`) — **not** a single `release` env.

---

## D. OIDC / trusted publishing by registry

### Shared GitHub OIDC facts

- Job needs `permissions: id-token: write` (job-scoped preferred).
- Issuer: `https://token.actions.githubusercontent.com`.
- Bind trust to **exact workflow filename**, repository, optional **environment**.
- Protect `release.yml` like a credential: third-party edits to that workflow ≈ token compromise (PyPI security model).

### Channel maturity

| Channel | OIDC / trusted publishing | First publish | Official action / notes |
|---------|---------------------------|---------------|-------------------------|
| **crates.io** | GA trusted publishing (GitHub; GitLab beta) | Manual API token once, then OIDC | `rust-lang/crates-io-auth-action@v1` → `CARGO_REGISTRY_TOKEN`; token ~30 min |
| **PyPI** | Mature Trusted Publishers | Pending publisher can create project | `pypa/gh-action-pypi-publish@release/v1`; short-lived API key ≤15 min |
| **npm** | Trusted publishing GA (2025-07) | Package often must exist / publisher configured; CLI npm ≥11.5.1, Node ≥22.14 | `id-token: write` + `npm publish`; auto provenance on public repo; **GitHub-hosted runners only** (no self-hosted yet) |
| **RubyGems** | Trusted publishing (since 2023) | Pending publisher for new gem | `rubygems/release-gem@v1`; environment recommended |

### Primary cites (OIDC registries)

| Source | URL | Scope used |
|--------|-----|------------|
| GitHub OIDC concept | https://docs.github.com/en/actions/concepts/security/openid-connect | Short-lived tokens; no long-lived cloud secrets |
| GitHub OIDC reference | https://docs.github.com/en/actions/reference/security/oidc | Claims, `id-token: write` |
| npm trusted publishers | https://docs.npmjs.com/trusted-publishers/ | Config fields, runner limits, provenance |
| npm OIDC GA changelog | https://github.blog/changelog/2025-07-31-npm-trusted-publishing-with-oidc-is-generally-available/ | GA date / requirements |
| PyPI using a publisher | https://docs.pypi.org/trusted-publishers/using-a-publisher/ | `pypi-publish` action |
| PyPI adding a publisher | https://docs.pypi.org/trusted-publishers/adding-a-publisher/ | owner/repo/workflow/environment |
| PyPI security model | https://docs.pypi.org/trusted-publishers/security-model/ | Workflow = credential; fork PR limits |
| crates.io trusted publishing | https://crates.io/docs/trusted-publishing/ | Setup, auth action, expiry |
| RFC 3691 | https://rust-lang.github.io/rfcs/3691-trusted-publishing-cratesio.html | Design / claims |
| RubyGems trusted publishing | https://guides.rubygems.org/trusted-publishing/ | Config + `release-gem` |
| RubyGems announcement | https://blog.rubygems.org/2023/12/14/trusted-publishing.html | Rationale |

---

## E. OS package channels — survey only (**OUT / Deferred**)

**Status:** These channels are **excluded from current release-channel scope** because they impose certificate signing, publisher identity, notarization, or long-lived package-repo GPG (or closely related external trust ops). Keep the survey for a future re-open decision; **do not** schedule them as Phase 3–4 deliverables under this research.

### winget — **EXCLUDE / Deferred**

- Catalog is **microsoft/winget-pkgs** manifests + public CDN — not an OIDC “trusted publisher” API like PyPI.
- Practical Windows distribution often collides with **Authenticode / SmartScreen / Microsoft Publisher Identity** expectations for installer trust, plus PAT+PR lag to winget-pkgs — unsuitable for near-term scope.
- Survey path (if re-opened later): `wingetcreate new` / `wingetcreate update … --submit` from **windows-latest** with a GitHub token secret; moderator review.

| Effort | Risk | Current scope |
|--------|------|---------------|
| Medium+ (manifest + PAT + PR lag + signing/trust UX) | Review delay; installer URL/hash drift; Authenticode/publisher-identity bar | **OUT / Deferred** |

Cites: https://learn.microsoft.com/en-us/windows/package-manager/package/repository ; https://github.com/microsoft/winget-create/blob/main/doc/update.md

### apt (Debian/Ubuntu) — **EXCLUDE / Deferred**

Three realistic patterns — **none** are GitHub→apt OIDC; all serious “apt channel” shapes need **GPG-signed** packages/repos (or equivalent trust ops):

| Pattern | Auth | Pros | Cons |
|---------|------|------|------|
| **Launchpad PPA** | GPG-signed source upload (`dput`) | Distro-native builds | GPG secret; Launchpad lag; series matrix |
| **Self-hosted apt on GitHub Pages** | GPG-signed `reprepro`/`aptly` repo | Full control; Actions-friendly | You own key rotation, Pages size, user trust UX |
| **OCI/GHCR apt transport** | Registry credentials / public pull | Novel; image-like | Client support uneven; still not classic apt UX |

**Current scope:** **OUT / Deferred** (GPG package/repo signing + hosting). **In-scope interim:** attach an **unsigned / checksum-listed** `.deb` (or tarball) to **GitHub Releases** without claiming an apt repository channel.

### Homebrew (brief) — **Deferred (non-cert)**

- Private/user **tap** (`brew tap-new`) that installs from checksummed GitHub Release assets does **not** by itself require Apple notarization / Developer ID.
- Still **out of near-term phases** (capacity); `homebrew/core` remains a separate high-bar review path.
- Bottles via tap GHA; publish with `brew pr-pull` — not registry OIDC. Do **not** fold Apple-notarized casks into “tap is easy.”
- Cite: https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap ; https://docs.brew.sh/Bottles

### Chocolatey (brief) — **EXCLUDE / Deferred**

- Community feed uses **API key** (`choco push`); Windows runner; moderation; package-signing / trust expectations for stronger listing.
- Overlaps Windows discovery with winget — exclude alongside winget for near-term scope.
- Cite: https://docs.chocolatey.org/en-us/create/commands/push/

### Authenticode / Apple notarization — **EXCLUDE / Deferred**

- **Windows Authenticode** signed MSI/EXE and **Apple Developer ID + notarization** for macOS installers require paid publisher/developer certificates and key-handling ops.
- Near-term Cross-OS artifacts: ship **unsigned** (or checksum-only) Release binaries; users install via OIDC language registries or direct Release download.

---

## F. One Rust binary → multi-language wrappers

arks should publish **native CLI binaries** (not require end users to install Rust). Language registries are **distribution UX**, not second engines.

| Ecosystem | Typical wrap | arks shape | Notes |
|-----------|--------------|------------|-------|
| **npm** | Platform-specific packages + root with `optionalDependencies` + thin JS `bin` shim; napi-rs pattern for native addons (CLI often uses pure binary packages without N-API) | `@arks/cli` + `@arks/cli-linux-x64` etc. **or** postinstall download from GitHub Release (worse: network at install) | Prefer **bundled** platform packages over postinstall fetch ([napi-rs release model](https://napi.rs/docs/deep-dive/release) as analogy) |
| **PyPI** | **maturin** `bindings = "bin"` → platform wheels; console_scripts entry | `arks` wheel per tag | [maturin](https://www.maturin.rs/) builds binaries as wheels; use manylinux/maturin-action for Linux |
| **RubyGems** | Platform gem shipping prebuilt binary, or thin gem + download; rb-sys more for extensions | Phase 2: platform gem wrapping same Release binary | Trusted publishing applies to `gem push` |
| **crates.io** | Source crate / binary crate via `cargo install` | Publish library/cli crates as normal Rust; **not** a substitute for end-user binary UX | OIDC for `cargo publish` |

**Binary artifact strategy**

1. Build once per triple in `dist` matrix (linux gnu/musl as needed, windows-msvc, darwin arm64/+x64).
2. Attach to GitHub Release with SHA256SUMS.
3. Wrapper publish jobs either (a) pack those bytes into npm/PyPI/gem artifacts, or (b) rebuild with identical toolchain pins — prefer (a) for bit-identical UX.
4. crates.io remains source-level; `cargo install arks-cli` is developer path.

---

## Recommended phased rollout

| Phase | Scope | Exit |
|------:|-------|------|
| **0** | Research + plan pointer (this doc); signing-constrained channels marked OUT | Specfold research landed |
| **1** | Cross-OS test matrix + MSRV + rust-cache + clean-room job in Actions | PR CI green; no registry publish |
| **2** | Tag → dist binaries → GitHub Release (**checksum / unsigned**); crates.io OIDC; PyPI wheels OIDC; npm platform packages OIDC; optional RubyGems OIDC | Install via cargo/npm/pip/(gem) documented |
| **3+** | Re-open only by explicit owner decision: Homebrew tap (non-cert) and/or previously **EXCLUDE**d signing channels (winget / apt GPG / Authenticode / notarization / Chocolatey) | N/A until re-opened |

**Removed from near-term phases:** former Phase 3 winget and Phase 4 apt/Chocolatey as in-scope deliverables — now **EXCLUDE / Deferred** per [Current release-channel scope](#current-release-channel-scope).

---

## What NOT to do

1. **Fake perf:** GHA timings as G19/Verified or public “faster than Git.”
2. **Long-lived publish tokens** for npm/PyPI/crates.io/RubyGems when trusted publishing is available.
3. **Self-hosted runners for npm OIDC** until npm documents support.
4. **Cache pollution of release publishes** (npm explicitly: disable package-manager cache on release).
5. **Claim Cross-OS verified** from Linux Docker alone (or claim Git-compat verified from Windows GHA alone).
6. **postinstall binary download** as the only npm/PyPI path when registry platform packages are feasible.
7. **Implement full workflow sprawl** before Phase 1 matrix + clean-room job exist.
8. **Silent dual SoT** for packaging — keep human docs thin; agent detail in `.spec/`.
9. **Treat apt / winget / Chocolatey / Authenticode / Apple notarization as current-scope** until owner re-opens signing-constrained channels.
10. **Conflate** registry OIDC provenance/attestation with Authenticode / Apple notarization / apt repo GPG.

---

## Open decisions (ask before SoT lock)

1. Public package names (`arks` vs scoped `@…/arks`) per registry trademark/availability.
2. Whether musl (Alpine) wheels/npm packages are in Phase 2 or later.
3. Whether unsigned Release `.deb` / `.msi` **assets** (not apt/winget channels) ship in Phase 2.
4. Homebrew user tap: re-open timing (non-cert) vs never until signing channels are reconsidered.
5. Single `release.yml` vs split `release-crates.yml` / `release-pypi.yml` (trusted publisher binds **filename** — splitting means multiple publisher configs).
6. Whether any future claim needs optional crates.io/npm **maintainer GPG** beyond OIDC provenance (default: **no** for near-term).

## Plan pointer

Implementation sequencing + Environment table: [[plan:root/20261110085731]]. Workflows: `.github/workflows/ci.yml`, `.github/workflows/release.yml`.
