---
id: sot:root/interop-test-matrix
title: Interop test matrix
status: active
created: 2026-11-08T08:21:48Z
updated: 2026-11-10T23:50:00Z
tags: [verification, interop, docker, clean-room, git, github, matrix, cli-parity, alias, server, journey, concurrency, issue-resolution, perf, m15, m16, m17, g20, g21, g22, g23, g24, g25, g26, g27, g28, g29, g30, g31, g32, g33, g34, g35, g36, g37, m23, m24, m25, m26]
verify: manual
rel:
  depends_on: [sot:root/product-verification-clean-room, sot:root/git-compat-surface, sot:root/remote-host-compatibility-surface, research:root/20261108073650, sot:root/cli-surface, sot:root/core-object-model, sot:root/crate-architecture, plan:root/20261108082148, plan:root/20261108082110, sot:root/storage-celir-boundary, sot:root/lease-concurrency-intent, sot:root/shared-index-nway-clean-room, sot:root/program-roadmap, research:root/20261108073228, research:root/20261108183800, sot:root/clean-room-journey-compare, sot:root/multi-agent-git-concurrency-scenarios, research:root/20261109023117, sot:root/cli-flag-parity-complete, plan:root/20261109043910, plan:root/20261109043916, adr:root/20261110074000, plan:root/20261110074000]
---

# Interop test matrix

Version **0.6.4**. Normative **sequencing** of stock-Git and remote interop gates **inside the Docker clean-room**. Authority and venue rules live in [[sot:root/product-verification-clean-room]]; dimension catalog in [[research:root/20261108073650]]. This SoT answers: **which gates unlock after which inner slices**, and what each gate must prove. Milestone / production mapping: [[sot:root/program-roadmap]] (**CLI-parity production historical = M9**; **full CLI inventory residual = M15 / G20**; **complete flag symmetry active alias = M16 / G21**; **production-server = M11 / G15**; **issue-resolution = M12/G17 + M13/G18**; **comparative-perf frozen allow-list = M14/G19**; **full G19 journeys = M17**; **L3 Claimed = M23 / G11-L3**; **CELIR index dual-path = M24 / G22–G25**; **M25** server transports **G29–G32**; **shared-index N-way = M26 / G34**). Additive journey-compare observability: [[sot:root/clean-room-journey-compare]] (**G16** — does not unlock/replace M9/M10/M11 and does **not** substitute for G17/G18 issue-resolution). Post-M24 additive Claimed gates: **G26** local PR-sim, **G27** arks smart-HTTP PR contention, **G28** live-private journeys + offline clone PASS (checklist N/A=0; land tip ~`902d728`). Residual additive Claimed: **G33** CRED, **G34** shared-index, **G35** `--depth` only (filter N/A), **G36** HTTP Basic (OAuth Refuse), **G37** gpgsig SSH (OpenPGP N/A).

## Rule

### Authority (inherited)

1. A gate that claims product “verified” for interop MUST pass in the pinned clean-room image/compose ([[sot:root/product-verification-clean-room]]). Host PATH Git is advisory only.
2. Stock-Git peers are the **in-image** Git pin — never the developer host binary.
3. GitHub-class remote smoke uses declared secrets/network or recorded redacted fixtures; never commit credentials ([[sot:root/remote-host-compatibility-surface]]).
4. Do not invent protocol or format assertions beyond [[sot:root/git-compat-surface]] / cited research.

### Unlock ladder (inner → outer)

Gates below are **locked** until their prerequisite slice exits. Slice ownership: core/inner SoTs + [[plan:root/20261108082110]]; outer sequencing [[plan:root/20261108082148]].

| Gate ID | Proves | Prerequisite slice | Clean-room entrypoint (intent) | Owning semantics |
|---------|--------|--------------------|--------------------------------|------------------|
| **G0** `unit-core` | Parsers / OID / pure object+ref+index helpers | Core object model crates compile | `cargo test` (and mirrors) inside clean-room image when release-cited | [[sot:root/core-object-model]] |
| **G1** `objects-roundtrip` | Loose object write/read interchangeable with stock Git | Core objects | In-image: `arks` hash-object/cat-file ↔ `git` hash-object/cat-file | git-compat objects |
| **G2** `refs-roundtrip` | Ref create/update/delete readable by stock Git | Refs after objects | In-image: `update-ref` / HEAD ↔ `git show-ref` / `git rev-parse` | git-compat refs |
| **G3** `index-commit-roundtrip` | Index + commit/tree stock-Git readable | Index + local commit path | In-image: `add`/`commit` then `git log`/`git ls-files`; reverse with `git` then `arks status`/`log` | index + objects |
| **G4** `pack-local-roundtrip` | Pack/idx produce/consume vs stock Git | Pack codec | In-image: pack-objects/index-pack (arks or lib harness) ↔ `git index-pack` / `git verify-pack` | packs |
| **G5** `protocol-offline` | pkt-line / v1|v2 fixture replay | Protocol client libs | Offline fixture replay in clean-room (no live network) | protocols research |
| **G6** `cli-local-foundation` | Wave **L** local CLI foundation ([[sot:root/cli-surface]]) | CLI wired on G1–G4 | Deterministic `arks` argv suite vs stock Git peer in clean-room | cli-surface |
| **G7** `remote-recorded` | Redacted remote-client HTTP/SSH session replay (GitHub-class **verify peer** preferred) | Protocol client + remote config | Recorded fixture replay in clean-room | remote-host (A) client |
| **G8** `remote-live-smoke` | Live clone/fetch/push to throwaway GitHub-class repo (verify peer) | G7 green + secrets (token + URL); force-off via `ARKS_G8_LIVE=0` | Clean-room online gate — **default-on when secrets**; honest SKIP when absent ([[adr:root/20261110220000]]) | remote-host client |
| **G9** `partial-shallow` | Claimed `--depth` subset (compose **G35**); filter/`blob:none`/`tree:0`/promisor remain N/A | Remote client claims depth | Clean-room matrix rows only for claimed `--depth` subset | remote research + [[sot:root/arks-remote]] |
| **G10** `storage-adapter` | Git-compat mode unchanged under storage facade | Storage adapter (Git backend first) | **Phased:** (a) after M2 — facade-over-loose-objects with **G1** still green; (b) after G2–G4 exist — those gates still green with adapter enabled. CELIR path only if ready SoT says so | [[sot:root/storage-celir-boundary]] |
| **G11** `lease-product` | Lease gates without breaking Git locks/compat / CLI-parity regression | Lease product slice (**M10**) | Dedicated clean-room services when lease SoT ships | [[sot:root/lease-concurrency-intent]] |
| **G11-L3** `lease-l3-stress` | L3 claimed-shape stress (N tens→hundreds ref-shard batch apply) + G11 regress; dual-path honesty | **M23** after [[adr:root/20261109194200]] | `verify/scripts/run-g11l3.sh` / compose `g11l3`; matrix `verify/matrices/matrix-l3.md` | [[plan:root/20261109194208]] |
| **G12** `cli-porcelain-pa` | Wave **Pa** command/flag suite vs stock Git | M7 / after G6+G7 | Clean-room argv + on-disk peer oracle for Pa inventory | cli-surface |
| **G13** `cli-porcelain-pb` | Wave **Pb** command/flag suite vs stock Git | M8 / after G12 | Clean-room argv + on-disk peer oracle for Pb inventory | cli-surface |
| **G14** `cli-parity-closure` | Full parity inventory + flag-parity audit + **alias-ready** (`git`→`arks`) vs pin; same-`.git` regression | M9 / after G13 | Clean-room matrix covering inventory (− exclusions) **and** alias-shim workflows; **CLI-parity production** gate | cli-surface + roadmap |
| **G15** `git-server` | Stock `git` clone/fetch/push against **arks as server** for claimed transports | **M11** / after G5 (+ G7 client regression preferred) | Clean-room: in-image stock Git client ↔ arks upload-pack/receive-pack (+ smart HTTP / claimed SSH) | remote-host server + roadmap |
| **G16** `journey-compare` | Side-by-side **user-journey** catalog: functional class + timing/perf metrics (stock Git vs arks; optional alias) | Additive after claimed inventory exists (prefer post-G14 for first-wave); **not** a production unlock; **not** issue-resolution Done | `verify/scripts/run-g16.sh` / compose `g16` | [[sot:root/clean-room-journey-compare]] |
| **G17** `concurrency-issue-compare-compat` | For each **claimed** compat catalog scenario ID (**M12** Cm-1..Cm-3 and **M19** Cx-1): **reproduce** induced fault under in-image stock **`git`** (pathology class holds) **and** under **`arks`** (SoT **resolved** class holds); comparative clean-room receipt | **M12** / **M19** after G14 + residual worktree/submodule Done; waves **Cm-1..Cm-3** Done; **Cx-1** active (`verify/matrices/matrix-g17.md`) | `verify/scripts/run-g17.sh` / compose `g17` | [[sot:root/multi-agent-git-concurrency-scenarios]], [[plan:root/20261109024634]], [[plan:root/20261110010000]], roadmap Issue-resolution Done rule |
| **G18** `concurrency-issue-compare-differentiate` | Same **reproduce + dual-peer compare** protocol as G17 for each **M13 claimed** `differentiate-beyond-git` / lease-uplift scenario ID | **M13** / after G11 + G17; claimed waves **Cd-1..Cd-3** Done (`verify/matrices/matrix-g18.md`) | `verify/scripts/run-g18.sh` / compose `g18` | [[sot:root/lease-concurrency-intent]], [[plan:root/20261109024635]], [[adr:root/20261109032955]] |
| **G19** `perf-comparative` | Claimed G16 journey allow-list: arks median wall-clock ≤ 80% of stock Git (**≥20% faster**), `median_p95`, release | **M14** frozen allow-list; **M17** expands to **all** current G16 journeys (`verify/matrices/matrix-g19.md`); only functionally green units | `verify/scripts/run-g19.sh` / compose `g19` | [[sot:root/clean-room-journey-compare]], [[plan:root/20261109024636]], [[plan:root/20261109043916]] |
| **G20** `cli-full-parity` | Each **M15 / Fs** claimed command/flag row: stock `git` ↔ `arks` peer functional PASS on pin | **M15** / after M9 historical; wave **Fs-1** (`verify/matrices/matrix-g20.md`) | `verify/scripts/run-g20.sh` / compose `g20` | [[sot:root/cli-surface]], [[sot:root/cli-flag-parity-full]], [[plan:root/20261109035335]] |
| **G21** `cli-complete-symmetry` | Each **M16 / Sy** claimed command/flag row: stock `git` ↔ `arks` peer functional PASS; no production refuse leftover on claimed options | **M16** / after M15 historical; waves **Sy-1..Sy-5** (`verify/matrices/matrix-g21.md`) | `verify/scripts/run-g21.sh` / compose `g21` | [[sot:root/cli-surface]], [[sot:root/cli-flag-parity-complete]], [[plan:root/20261109043910]] |
| **G22** `celir-index-contention` | N-way (32/128) concurrent agents: CELIR index build/publish/open/query + rebuild-from-`.git` under leases + FS `.git`; integrity / fail-closed / stock-readable; ObjectStore Refuse | **M24** after feature ship; [[sot:root/celir-index-contention-clean-room]] | `verify/scripts/run-g22.sh` / compose `g22`; matrix `verify/matrices/matrix-g22.md` | [[sot:root/celir-index-dual-path]], [[plan:root/20261109203949]] |
| **G23** `celir-dual-path-git` | Same-repo FS `.git` porcelain/interop + celir-index build/publish/open/query; stock Git open; **CELIR dual-path Git verified** (not ObjectStore) | After G22; [[sot:root/celir-dual-path-git-verified]] + [[adr:root/20261109204916]] | `verify/scripts/run-g23.sh` / compose `g23`; matrix `verify/matrices/matrix-g23.md` | [[plan:root/20261109204933]] |
| **G24** `l3-celir-index-joint` | WriterMode::HighConcurrency + CELIR index together; locks still taken; shared-index N-way OUT | After G22 + M23 L3; [[sot:root/l3-celir-index-joint-clean-room]] | `verify/scripts/run-g24.sh` / compose `g24`; matrix `verify/matrices/matrix-g24.md` | [[plan:root/20261109204935]] |
| **G25** `celir-index-scale` | G22 scale extension: N=256 required; 384/512 stretch under sole-runner OOM policy | After G22; [[sot:root/celir-index-scale-clean-room]] | `verify/scripts/run-g25.sh` / compose `g25`; matrix `verify/matrices/matrix-g25.md` | [[plan:root/20261109204934]] |
| **G26** `local-branch-pr-sim` | Thousands-class local branches + synthetic `refs/pull/N/{head,merge}` + leased concurrent tip/merge workers; fail-closed; stock pathology compare | Additive local PR-env sim; [[sot:root/local-branch-pr-sim-clean-room]] | `verify/scripts/run-g26.sh` / compose `g26`; matrix `verify/matrices/matrix-g26.md` | [[plan:root/20261109205507]]; GitHub live Deferred (G8+) |
| **G27** `arks-smart-http-pr-contention` | Thousands-class branches + synthetic `refs/pull/N/{head,merge}` on **arks smart HTTP**; N concurrent stock-Git clone/fetch/push; contended same-ref fail-closed | Additive after G15 Claimed; GitHub stand-in (not live forge); [[sot:root/arks-smart-http-pr-contention-clean-room]] | `verify/scripts/run-g27.sh` / compose `g27`; matrix `verify/matrices/matrix-g27.md` | [[plan:root/20261109210427]]; ≠ GitHub UX/Checks/merge-queue; live Deferred (G8+) |
| **G28** `live-private-arks-journeys` | G16-derived **normal-user journeys** with **arks** against throwaway **private** GitHub (clone/fetch/push/pull, branch, merge, tag, stash, worktree/submodule, PR); offline **J-CLONE-LOCAL-01** / **J-CLONE-PATH-01** via additive compose | Opt-in secrets (`ARKS_G28_LIVE=1` + token + URL); explicit user unlock; default Refuse-closed; offline clones need no secrets | `verify/scripts/run-g28.sh` / compose `g28` + `g28-offline-clones`; matrix `verify/matrices/matrix-g28.md` | [[sot:root/g28-live-private-arks-journeys]], [[plan:root/20261109212020]]; ≠ scale/contention; ≠ G16 timing |
| **G29** `server-dumb-http` | Stock `git` clone/fetch against arks **dumb HTTP** (additive; smart HTTP still Claimed on G15) | **M25** after [[adr:root/20261109220230]]; smart HTTP required as non-sole path | `verify/scripts/run-g29.sh` / compose `g29`; matrix `verify/matrices/matrix-g29.md` | [[sot:root/arks-server]], [[plan:root/20261109220230]] |
| **G30** `server-ssh-command` | Stock `git` clone/fetch/push via **SSH command-mode** shape (duplex `arks upload-pack` / `arks receive-pack`) | **M25** after unlock ADR | `verify/scripts/run-g30.sh` / compose `g30`; matrix `verify/matrices/matrix-g30.md` | [[sot:root/arks-server]], gitprotocol-pack |
| **G31** `server-protocol-v2` | Stock `git` clone/fetch with protocol **v2** (`ls-refs` + `fetch`) against arks server | **M25** after unlock ADR | `verify/scripts/run-g31.sh` / compose `g31`; matrix `verify/matrices/matrix-g31.md` | [[sot:root/arks-server]], [gitprotocol-v2](https://git-scm.com/docs/gitprotocol-v2) |
| **G32** `server-git-daemon` | Stock `git` clone/fetch via **`git://`** against `arks daemon` | **M25** after unlock ADR | `verify/scripts/run-g32.sh` / compose `g32`; matrix `verify/matrices/matrix-g32.md` | [[sot:root/arks-server]], [git-daemon](https://git-scm.com/docs/git-daemon) |
| **G33** `client-cred-askpass` | HTTPS Basic via `arks credential fill` + `credential.helper` + `GIT_ASKPASS` + TTY PTY + first-party `wincred`/`libsecret`/`osxkeychain` (file wallet dir in clean-room) against auth-required loopback smart HTTP | After G7; [[sot:root/credential-helper-askpass]] unlock for **FW-CRED-ASKPASS** | `verify/scripts/run-g33.sh` / compose `g33`; matrix `verify/matrices/matrix-g33.md` | [[sot:root/credential-helper-askpass]], [gitcredentials](https://git-scm.com/docs/gitcredentials), [git-credential](https://git-scm.com/docs/git-credential) |
| **G34** `shared-index-contention` | N≥2 writers on one shared `$GIT_DIR/index` via `index:shared` lease + `index.lock`; fail-closed; stock-readable; ≠ G22/G24 | **M26** after [[adr:root/20261110074000]] | `verify/scripts/run-g34.sh` / compose `g34`; matrix `verify/matrices/matrix-g34.md` | [[sot:root/shared-index-nway-clean-room]], [api-lockfile](https://git-scm.com/docs/api-lockfile), [gitformat-index](https://git-scm.com/docs/gitformat-index) |
| **G35** `client-shallow-depth` | arks `--depth N` clone/fetch vs stock Git peer: `.git/shallow` boundary + tip checkout; no invented filter caps | **G9** Claimed `--depth` subset after G7 client path | `verify/scripts/run-g35.sh` / compose `g35`; matrix `verify/matrices/matrix-g35.md` | [[sot:root/arks-remote]], [git-clone `--depth`](https://git-scm.com/docs/git-clone#Documentation/git-clone.txt---depthltdepthgt), [gitprotocol-capabilities](https://git-scm.com/docs/gitprotocol-capabilities) `shallow` |
| **G36** `server-http-basic` | Stock `git` clone against arks smart HTTP with optional in-process Basic; 401 honesty; unauthenticated G15 path remains | After G15; [[adr:root/20261109224236]] | `verify/scripts/run-g36.sh` / compose `g36`; matrix `verify/matrices/matrix-g36.md` | [[sot:root/arks-server]]; forge OAuth Refuse |
| **G37** `gpgsig-ssh-create-verify` | `commit -S` / `verify-commit` for `gpg.format=ssh`; refuse OpenPGP format; no invented tag -s | After object/`gpgsig` read path | `verify/scripts/run-g37.sh` / compose `g37`; matrix `verify/matrices/matrix-g37.md` | [[sot:root/arks-object]], [gitformat-signature](https://git-scm.com/docs/gitformat-signature); OpenPGP/X.509 N/A |

**Issue-resolution compare protocol (G17 / G18)**

1. Claim-set is explicit in the staged plan / matrix file; unlisted catalog IDs are **Open** (not resolved).
2. For each claimed ID: induce the scenario; stock `git` lane MUST match the SoT “exhibits pathology” oracle; `arks` lane MUST match the SoT “resolved” oracle.
3. Slice Done = **every** claimed ID has a clean-room PASS receipt. SoT prose / host demos are insufficient.
4. Timings MAY be recorded; absolute / ≥20% pass-fail is **G19 only**.

**Ordering constraints**

- Do not run G6 before G1–G4 for the features those commands claim.
- Do not run G7/G8 before G5 for the transport under test.
- Do not run G12 before G6 (and G7 when Pa commands need remotes).
- Do not run G13 before G12; do not run G14 before G13.
- G8 MUST NOT be the sole remote proof if offline G7 exists; prefer G7 as CI default when live creds absent (open decision under clean-room SoT).
- G10 is **phased**: M2 may claim **G10 objects-subset** (facade-over-loose-objects + G1 regression) before refs/pack exist. Full G1–G4-through-facade regression runs only after those gates exist (typically post-M4 / with M5 regression). CELIR-backed G10 is blocked until storage boundary marks ready.
- Full **G3** (index-commit round-trip) unlocks with local commit porcelain (**M5**); M3 may prove **index r/w** only without claiming full G3.
- G11 never unlocks before CLI-parity production gates for the same repo mode stay green (**after M9** by default).
- **G15** never unlocks before protocol offline (**G5**) for the claimed transport; prefer G7 client regression green; production-server claims require **G15** at **M11** (not M9).
- **G6/G7 green ≠ production.** CLI-parity production requires **G14** (+ G0–G7, G12–G13 regression) at **M9**. **G15** is production-server only.
- **G16** is **observability / comparison evidence**, not an unlock substitute for G14/G11/G15 and not a production or issue-resolution milestone. Absolute ms / ≥20% pass-fail bars remain deferred to **G19 / M14** and MUST NOT redefine M9/M10/M11 or substitute for G17/G18.
- **G17** never unlocks before G14 (CLI-parity) for the repo modes under test; host-only concurrency demos do not authorize G17.
- **G18** never unlocks before G11 + G17 for claimed differentiate rows that depend on leases + compat baseline.
- **G19** never unlocks before **both** G17 (M12 claimed set) and G18 (M13 claimed set) are green for their accepted claim-sets and M14 is staged; MUST NOT be treated as issue-resolution Done; MUST NOT authorize parallel perf-opt Done beside open Cm/Cd; MUST NOT redefine **G20**/M15 or **G21**/M16 CLI symmetry. **M17** expands the G19 allow-list to all G16 journeys after functional green units.
- **G20** never unlocks before G14 historical green; Fs wave Done = every claimed ID in that wave’s matrix PASS; host greens do not authorize M15 exit.
- **G21** never unlocks before G20 historical green (M15 exit); Sy wave Done = every claimed ID in that wave’s matrix PASS; host greens do not authorize M16 exit; production refuse leftovers on claimed options are defects.
- **G22** never unlocks before M24 feature ship (host advisory Claimed OK); MUST NOT redefine G11-L3; MUST NOT claim CELIR ObjectStore; host greens do not authorize index-contention verified; discrete-circuit sibling mount required or FAIL.
- **G23** never unlocks before G22; authorizes **CELIR dual-path Git verified** only with receipt ([[adr:root/20261109204916]]); ObjectStore Refuse retained; host greens insufficient.
- **G24** never unlocks before G22 + M23 L3 Claimed; MUST NOT redefine G11-L3 as CELIR-bearing; lease ≠ skip locks; G24 is **not** shared-index N-way proof.
- **G25** never unlocks before G22; N=256 required; stretch OOM = hard-block for that tier only (no fake green).
- **G33** Claimed paths include TTY + first-party OS wallet helpers; forge OAuth remains out; host greens are advisory only.
- **G34** never unlocks before M26 unlock ADR; MUST NOT substitute G22/G24; MUST NOT skip `index.lock`; host greens are advisory only.
- **G35** proves `--depth` only; MUST NOT invent filter/`blob:none`/`tree:0`/promisor claims.
- **G36** proves product Basic when configured; forge OAuth remains Refuse; MUST NOT redefine G15 unauthenticated path.
- **G37** proves `gpg.format=ssh` only; OpenPGP/X.509/`tag -s` remain N/A.
- G8 live-remote is **default-on when secrets** and honest SKIP when absent ([[adr:root/20261110220000]]); force-off via `ARKS_G8_LIVE=0` — MUST NOT redefine M9/M10/M11 ([[sot:root/production-residual-register]]).
- Live GitHub **forge scale** (PR throughput / rate limits) is **Claimed (harness ready)** — measure when secrets; SKIP when absent — `verify/matrices/matrix-g8-live-scale.md` ([[adr:root/20261110221000]]); ≠ G28 functional journeys.

### Stock Git scenarios (inside clean-room)

Minimum scenarios when the corresponding gate is unlocked:

| Scenario | Gate | Sketch |
|----------|------|--------|
| Blob OID parity | G1 | Same bytes → same OID via arks and `git hash-object` |
| Commit graph readable | G3 | arks commit; `git log --oneline` shows it; reverse |
| Ref advertise consistency | G2 | Branch tip matches across tools |
| Pack interchange | G4 | Pack written by one, indexed/verified by the other |
| Refuse unknown `extensions.*` | G1–G3 as applicable | arks refuses repo stock Git marks version=1 with unknown extension |

### Remote client scenarios (inside clean-room; GitHub-class verify peer)

| Scenario | Gate | Sketch |
|----------|------|--------|
| Recorded smart HTTP fetch | G7 | Replay scrubbed `info/refs` + POST upload-pack |
| Recorded push report-status | G7 | Replay receive-pack path |
| Live clone HTTPS | G8 | Throwaway GitHub-class repo; PAT/helper from CI env |
| Live push new branch | G8 | Push then fetch tip; scrub logs |
| Partial/shallow (if claimed) | G9 | Only documented filters; cite remote research |

### Git server scenarios (inside clean-room — M11 / G15)

| Scenario | Gate | Sketch |
|----------|------|--------|
| Stock `git clone` via smart HTTP | G15 | Clean-room Git pin clones from arks smart HTTP |
| Stock `git fetch` / `git push` | G15 | Round-trip refs/objects; report-status as claimed |
| Capability refuse | G15 | Unimplemented caps not advertised; baseline still works |
| SSH / `git://` (only if claimed) | G15 | Same clone/fetch/push matrix on that transport |

### Host-advisory (non-unlocking)

Unit tests, formatters, and laptop `git` experiments MAY run on the host. They do **not** unlock G1–G15 (and do not authorize G16 comparison claims) and MUST NOT be cited as verified without a clean-room receipt ([[research:root/20261108073650]]).

### Recipe naming (design; paths still open)

Until [[sot:root/product-verification-clean-room]] closes compose/layout decisions, gate IDs above are the stable names. Future compose services SHOULD map 1:1 (e.g. `gate-g3-index-commit`). Do not invent image digests here.

## Why

Venue rules alone do not tell an implementer **when** to turn on stock-Git, remote-client, or server tests. Unlock sequencing prevents outer CLI/remote greens that hide broken object/ref/pack kernels, keeps porcelain-wave gates (G12–G14) from being skipped for a false “production,” keeps **G15** from redefining M9, and keeps CELIR/lease product gates from redefining CLI-parity success. Clean-room execution keeps “verified” reproducible.

## Verification

1. `spec check` clean for this SoT and linked clean-room / matrix research.
2. Each new interop test PR names a Gate ID and prerequisite slice; CI labels clean-room vs host-advisory.
3. Closing a clean-room Open decision updates recipes without renaming Gate IDs unless this SoT is revised.
4. No gate marked verified from host-only runs.
