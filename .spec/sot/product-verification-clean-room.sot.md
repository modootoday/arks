---
id: sot:root/product-verification-clean-room
title: Product verification clean-room
status: active
created: 2026-11-08T07:36:50Z
updated: 2026-11-10T09:53:55Z
tags: [verification, docker, clean-room, ci, git-compat, public, journey, layout]
verify: manual
rel:
  depends_on: [sot:root/git-compat-surface, sot:root/remote-host-compatibility-surface, research:root/20261108073650, sot:root/clean-room-journey-compare, sot:root/structural-conventions]
  evidence: [research:root/20261108073650, research:root/20261110095355]
---

# Product verification clean-room

Version 0.1.4. Normative **intent** for how GIR claims “verified.” Any product acceptance, interop, or regression gate that asserts verified status MUST be reproducible in a **Docker clean-room**. Host-local runs are development aids only. Convention affinity (Docker as completion authority; host e2e not “done”) follows humanymous patterns in `C:\workspaces\humanymous\.agents\rules\60-e2e-docker-only.md` and `96-docker-engine-affinity.md` — cite those rules; do not fork competing convention prose in this repo. Tree grouping vs crate `tests/`/`testdata/`: [[research:root/20261110095355]], [[sot:root/structural-conventions]] §7.

Compatibility semantics remain owned by [[sot:root/git-compat-surface]] (formats/protocols) and [[sot:root/remote-host-compatibility-surface]] (GitHub-class remotes). Matrix dimensions and host-vs-clean-room placement: [[research:root/20261108073650]]. Additive journey-compare (func + timing) receipts: [[sot:root/clean-room-journey-compare]] (**G16**).

## Rule

### Authority

1. **Clean-room Docker is authoritative.** A claim that GIR is verified for a product gate (acceptance, Git interop, GitHub-class remote smoke, or future lease/concurrency product gates) is valid only when that gate has passed inside the clean-room image/compose path defined for that gate.
2. **CI uses the same images/compose.** Continuous integration MUST invoke the same pinned images and compose (or equivalent deterministic entrypoint) as local clean-room verification. Divergent CI recipes that only run host toolchains MUST NOT be labeled product-verified.
3. **Host tests are advisory.** Unit/integration runs on the developer host (installed Git, language toolchains, PATH extras, Ruby/Python helpers, IDE runners) MAY speed iteration. They are **non-authoritative** unless the same suite is mirrored and green in clean-room. Host-only green does not satisfy “done.”
4. **No silent host pollution.** Clean-room runs MUST NOT depend on host-installed Git extras, host PATH pollution, or host-local language runtimes unless a gate explicitly declares a bind-mount or volume and records that exception in the gate’s evidence.

### Clean-room definition (normative)

A **clean-room** verification environment is all of the following:

| Property | Requirement |
|----------|-------------|
| **Fresh container** | Each authoritative run starts from a new container (or compose project) from the pinned image set — not a long-lived dirty workspace reused across unrelated gates. |
| **Pinned base image** | Base and tool images are pinned by tag **and** digest (or a companion lock file the SoT points at). Floating `latest` is forbidden for authoritative gates. |
| **No undeclared host toolchains** | Do not bind-mount host `/usr`, host Git installs, or host language homes into the container for authoritative runs. Declared exceptions (e.g. read-only fixture mounts, secret files from CI) MUST be listed in the gate recipe. |
| **Deterministic entrypoint** | The gate is entered via a fixed script/compose service command (same argv shape in CI and local). Ad-hoc `docker run ... bash` explorations are non-authoritative. |
| **In-image stock Git** | Round-trips that claim stock-Git interop use the **Git binary inside the clean-room image**, at the pinned version — not whatever `git` is on the host. |
| **In-image remotes peers** | GitHub-class remote smoke that claims verified uses credentials/network only as declared by the gate; recorded/offline fixtures used for regression still execute inside clean-room. |

### What must run in clean-room

When the corresponding product surface ships, these gates MUST be clean-room authoritative (details in [[research:root/20261108073650]]):

- Product acceptance / release readiness aggregates that claim verified.
- Git-compat fixture round-trips against stock Git ([[sot:root/git-compat-surface]] Verification intent).
- GitHub-class remote client smoke / recorded remote regression ([[sot:root/remote-host-compatibility-surface]] test-plan sketch).
- Lease/concurrency product gates when claimed ([[sot:root/lease-concurrency-intent]]) and CELIR-backed store gates when readiness flips ([[sot:root/storage-celir-boundary]]) — same authority rule; no special host exemption. Core build slices list per-slice acceptance in [[plan:root/20261108082110]].

### What may run on the host (advisory)

- Fast unit tests and formatters/linters during edit loops.
- Authoring fixtures before packaging them into the clean-room image or fixture volume.
- Exploratory `git` / debugger sessions on the host.

Advisory results MUST NOT be cited as product verification in READMEs, release notes, Pages (`docs/`), or Specfold evidence without a matching clean-room receipt. Public-surface placement for such claims: [[sot:root/public-surface]].

### Image and lock intent (M1–M4 harness)

5. **Pin digests/tags in companion lock.** Authoritative image identity lives in `verify/images.lock.json` (referenced here). Until a gate’s digest is recorded there, that gate remains **manual / not yet harnessed**; do not invent digests. M1–M4 reuse the same `arks-verify-m1:0.1.0` image tag until a digest bump is recorded.
6. **Compose / tree layout (grouped):** `verify/` owns product clean-room only:
   - Harness entry: `docker-compose.yml`, `Dockerfile`, `images.lock.json`, `scripts/run-g*.sh` (incl. journey `run-g16.sh` / comparative `run-g19.sh`).
   - Matrices: `verify/matrices/matrix-*.md` (not a flat dump at `verify/` root).
   - Gate oracles / receipts / locks: `verify/gates/gN/` (out-of-workspace Cargo packages when needed; not Cargo workspace members).
   - Shared oracle intent/bytes: `verify/fixtures/` (no secrets).
   - Single-crate integration that is only host-advisory MAY live under `crates/arks-<ctx>/tests/` and be invoked by a thin `scripts/run-gN.sh` (G34 model). Compose service `g4` enters `run-g4.sh` (embeds G5 + regressions); `g5` enters `run-g5.sh`; `g6` enters `run-g6.sh`; `g3` enters `run-g3.sh`; `g8` enters `run-g8.sh`; `g16` enters `run-g16.sh`; `g19` enters `run-g19.sh`.
7. **Engine affinity on Windows.** When agents or operators run clean-room locally on Windows, follow humanymous Docker engine affinity: Windows Docker Desktop Linux engine (`desktop-linux`), no silent WSL-daemon substitution. Recorded as convention cite, not a second affinity SoT.

### Forbidden as “verified”

- Host-only stock-Git round-trips as the sole interop gate.
- CI jobs that install Git/tooling on the runner OS and never enter the pinned compose/image.
- Claiming GitHub remote compat verified from a laptop PATH Git without clean-room mirror.
- Floating image tags without digest/lock for release gates.
- Using gitoxide/`gix` (or any non-stock-Git library) as a substitute compatibility oracle for product “verified” claims — stock Git in-image remains the **only** peer ([[adr:root/20261108083507]]). Permanent optional/`dev-dependencies` `gix` is not an accepted verification design; workspace end state is zero `gix`.

### Open decisions (tracked here until closed)

| Decision | Status | Notes |
|----------|--------|-------|
| Base image family | **Closed (M1)** | Official `rust:*-bookworm` + apt `git`; digest in `verify/images.lock.json` |
| Stock Git pin | **Closed (M1)** | Debian bookworm apt Git recorded in lock (`stock_git.expected_version_output`); bump lock + this SoT together |
| Compose / tree layout | **Closed (grouped)** | `verify/` with `scripts/`, `matrices/`, `gates/gN/`, `fixtures/` ([[research:root/20261110095355]]) |
| Image lock file path | **Closed (M1)** | `verify/images.lock.json` |
| Network policy for live GitHub smoke | **Claimed (default-on when secrets)** | Default authoritative path = offline **G7**; G8 live runs when token + throwaway URL present (`ARKS_G8_LIVE=1` not required); honest SKIP when absent per [[adr:root/20261110220000]] |

Closing a row means updating this SoT (and the lock/compose when implemented), not chat-only resolution.

## Why

GIR’s mission is Git-compatible replacement with GitHub-class remote interop. Host PATH Git, Ruby helpers, and IDE-bundled tools create false greens that fail on clean CI or peer machines. Humanymous already treats Docker as product completion authority; GIR inherits that discipline for verification so “verified” means a reproducible container recipe, not a developer laptop. Pinning Git inside the image keeps stock-Git round-trips honest against [[sot:root/git-compat-surface]] and remote smoke honest against [[sot:root/remote-host-compatibility-surface]].

## Verification

1. `spec check` clean for this SoT and [[research:root/20261108073650]].
2. M1–M4 gates: `docker compose -f verify/docker-compose.yml run --rm g0` / `g1` / `g10` / `g2` / `g4` / `g5` with digests matching `verify/images.lock.json`. Journey-compare: `run --rm g16` per [[sot:root/clean-room-journey-compare]] (timings authoritative only in clean-room).
3. Any PR that adds a product gate labels it **clean-room** or **host-advisory** per the matrix research; clean-room gates name the intended compose/service.
4. Same image digest/tag in local clean-room and CI; stock Git version printed in gate evidence matches the lock.
5. Do not mark release or interop “verified” from host-only runs.
6. Closing Open decisions updates this SoT; invented base images or Git versions without a lock entry are non-compliant.
