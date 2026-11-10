---
id: sot:root/arks-remote
title: arks-remote unit
status: active
created: 2026-11-08T11:05:00Z
updated: 2026-11-10T08:00:00Z
tags: [m6, m16, unit, remote, client, smart-http, ssh, ring-6, phase-h, local-clone, g9, g35, shallow]
verify: manual
rel:
  governs: [crates/arks-remote]
  depends_on:
    - sot:root/credential-helper-askpass
    - sot:root/program-roadmap
    - plan:root/20261108090651
    - sot:root/remote-host-compatibility-surface
    - sot:root/cli-surface
    - sot:root/git-compat-surface
    - sot:root/crate-architecture
    - sot:root/arks-protocol
    - sot:root/arks-pack
    - sot:root/arks-repo
    - sot:root/arks-refs
    - sot:root/arks-store
    - sot:root/arks-object
    - sot:root/interop-test-matrix
    - sot:root/product-verification-clean-room
    - sot:root/storage-celir-boundary
    - sot:root/clean-room-journey-compare
    - research:root/20261108071916
    - research:root/20261108073228
    - adr:root/20261108083507
    - evidence:root/20261108104837
  evidence: [evidence:root/20261108112000, evidence:root/20261110180000]
---

# arks-remote unit

Version **0.4.0**. Unit SoT for Ring 6 Git **remote client** (`crates/arks-remote`) under [[plan:root/20261108090651]] / roadmap Phase **H** (**M6**) + additive **G9/G35** shallow. Wire bytes cite [gitprotocol-http](https://git-scm.com/docs/gitprotocol-http), [gitprotocol-pack](https://git-scm.com/docs/gitprotocol-pack), [gitprotocol-common](https://git-scm.com/docs/gitprotocol-common), [gitprotocol-capabilities](https://git-scm.com/docs/gitprotocol-capabilities) (`shallow` / `deepen`); catalog [[research:root/20261108071916]] + [[research:root/20261108071917]]. Host verify-peer policy: [[sot:root/remote-host-compatibility-surface]].

**Entry:** M5 exit proven — [[evidence:root/20261108104837]]. **Not production** — production is **M9**.

## Rule

### Role

1. **Generic client.** Speak claimed smart HTTP so **any** peer offering the claimed services interoperate for clone/fetch/push. GitHub-class hosts are the **representative verify peer**, not an exclusive remote type.
2. **Surface (A) only.** No forge REST/GraphQL / Apps / Checks (scope B).
3. **Client only.** No upload-pack/receive-pack **server** product path (that is **M11** / `arks-server`).
4. **Inward deps only.** MAY depend on `arks-protocol`, `arks-pack`, Rings 0–4 (`arks-hash`/`arks-oid`/`arks-object`/`arks-store`/`arks-refs`/`arks-repo` / index as needed). MUST NOT depend on `arks` CLI types, `arks-server`, CELIR, or lease L1+ runtime. **Zero `gix`**.

### Claimed transport subset (M6 + additive local)

| Transport | Claim | Notes |
|-----------|-------|-------|
| Smart HTTP (`info/refs` + service POST) over `http://` and `https://` | **Required** | Same request/response shapes; TLS for `https://` |
| Local filesystem **path** and same-host `file://` (absolute; empty host or `localhost`) | **Required (v0.2.0 additive; v0.3.0 pack-aware; v0.4.0 hardlink-prefer)** | **Clone composition only** via local object-directory **hardlink-or-copy** (prefer hardlink when the filesystem allows — git-clone(1) local default; fall back to byte copy); packs kept in place and read through pack-aware [[sot:root/arks-store]]. Cite [git-clone(1)](https://git-scm.com/docs/git-clone) local / `file://` forms; [[research:root/20261108071916]] file-transport note. **Not** `--shared` alternates; **not** piped upload-pack over `file://`. Does **not** unlock G8 |
| SSH (`git@` / `ssh://`) | **Required (M16 Sy-4)** — OpenSSH `ssh` helper speaking upload-pack/receive-pack pkt-line (no product-path `git`) | [[sot:root/cli-flag-parity-complete]]; auth via agent / key files (OpenSSH defaults); askpass / credential-helper N/A |
| Dumb HTTP | **Not claimed** | Refuse / do not advertise |
| `git://` | **Not claimed** | — |

### Operations (MUST for claimed subset)

5. **Remote config.** Read/write `[remote "<name>"]` `url` and `fetch` refspec lines in `.git/config` as Git documents for the claimed subset (add / list / remove).
6. **Fetch / clone path (HTTP).** `GET …/info/refs?service=git-upload-pack` then `POST …/git-upload-pack`; parse v1 advertisement; send want/have/done; demux side-band when negotiated; decode pack via `arks-pack`; **ingest as pack+idx** (prefer over exploding every object to loose); update remote-tracking refs per fetch refspec.
6a. **Fetch / clone path (SSH).** OpenSSH `ssh` (or `GIT_SSH_COMMAND` / `ARKS_SSH_COMMAND`) runs peer `git-upload-pack '<path>'`; speak gitprotocol-pack v1 advertisement + want/have/done over the duplex; demux side-band when negotiated; ingest pack+idx; update tracking refs. Cite [[research:root/20261108071916]] SSH/pack note.
7. **Push path (HTTP).** `GET …/info/refs?service=git-receive-pack` then `POST …/git-receive-pack`; send update commands + pack; parse `report-status` / `report-status-v2` as advertised for the claimed subset.
7a. **Push path (SSH).** Same pack-protocol push dialog over OpenSSH running peer `git-receive-pack '<path>'`.
8. **Clone composition (HTTP).** Init repo layout + fetch + baseline checkout of HEAD tree into the work tree (and index) for the claimed non-bare case.
8a. **Clone composition (local path / `file://`).** Resolve path; discover source `$GIT_DIR`; init dest layout; hardlink-or-copy `objects/` from source (prefer hardlink; packs kept; pack-aware store); copy heads/tags + set `HEAD` / `origin` tracking; baseline checkout. Refuse non-local `file://` hosts and non-path schemes outside this table.
8b. **Clone composition (SSH).** Init + SSH fetch + baseline checkout (same as HTTP composition after objects arrive).
9. **Capability honesty.** Prefer protocol v1 for M6 product claim (v2 MAY be parsed later). Never advertise unimplemented caps on the client want line beyond the negotiated subset. Unknown optional peer caps MAY be ignored.
10. **Auth.** HTTPS: optional credentials via URL userinfo; `ARKS_REMOTE_USERNAME`/`ARKS_REMOTE_PASSWORD`; configured `credential.helper` (including first-party `wincred` / `libsecret` / `osxkeychain`); `GIT_ASKPASS`/`core.askPass`/`SSH_ASKPASS`; and TTY terminal prompt when `GIT_TERMINAL_PROMPT`≠`0` — [[sot:root/credential-helper-askpass]]. No forge OAuth UX. SSH: OpenSSH agent / key files / `GIT_SSH_COMMAND`. Secrets MUST NOT enter the tree or fixtures. Recorded/local peers MAY need no auth.
11. **Pull.** Incomplete `pull` (fetch+merge without Pa merge) MUST refuse non-zero without mutating remote state. SSH URLs are claimed under Sy-4 (not refuse).

### Claimed shallow subset (G9 / clean-room **G35**; v0.4.0)

12. **`--depth N` only.** Client MAY request positive commit depth on smart HTTP and SSH clone/fetch when the peer advertises the `shallow` capability ([gitprotocol-capabilities](https://git-scm.com/docs/gitprotocol-capabilities) `shallow`; [git-clone `--depth`](https://git-scm.com/docs/git-clone#Documentation/git-clone.txt---depthltdepthgt)). Wire: request `shallow` on the first want line; send `deepen N`; parse server `shallow` / `unshallow` lines; write `$GIT_DIR/shallow` (gitrepository-layout). Existing local shallow OIDs MUST be re-sent as client `shallow` lines on later fetches.
13. **Capability honesty.** MUST NOT request `shallow` / `deepen` / `filter` unless implemented and peer-advertised. Depth against a peer that does not advertise `shallow` MUST refuse. Local path / `file://` clone MUST refuse `--depth` (no pack-protocol deepen on the claimed local object-dir path).
14. **Non-claims (remain out).** Partial-clone `filter` (`blob:none`, `tree:0`), promisor remotes, `extensions.partialClone`, `deepen-since` / `deepen-not` / `deepen-relative`, `--unshallow`, server-side shallow product. Clean-room proof: [[sot:root/interop-test-matrix]] **G9** semantics via compose **G35** (`verify/matrix-g35.md`) — coordination: G33=CRED, G34=SHARED-IDX.

### Out of scope (M6)

- Git server role / `arks-server` (**M11**).
- Forge product APIs (B).
- CELIR; lease L1+ runtime (**M10**).
- Partial-clone filter / promisor product claims (beyond the `--depth` subset above).
- Calling M6 **production**.
- Permanent `gix`; product-path `git` subprocess (stock Git allowed only as clean-room oracle / local peer fixture).
- `--shared` / alternates local clone; piped `file://` upload-pack clone (may amend later). Hardlink-prefer for object files is claimed (v0.4.0).
- Redefining **M9** / **M10** / **M11**.

## Why

Wave H unlocks remotes before porcelain waves. A Ring 6 client over proven pkt-line/pack libraries keeps wire semantics standards-first and unlocks clean-room **G7** without waiting for M11 server or forge APIs.

## Verification

1. Host (advisory): `CARGO_BUILD_JOBS=2 cargo test -p arks-remote` and Wave H CLI tests via `-p arks`.
2. Clean-room **G7** `remote-recorded` (+ G1–G6 regression) per `verify/matrix-m6.md` — authoritative. **G8** default **Refuse-closed** without opt-in secrets ([[adr:root/20261110151000]]); live only when `ARKS_G8_LIVE=1` + token + URL. Host green alone is not product-verified.
3. Clean-room **G35** (`verify/matrix-g35.md`, compose `g35`) for the claimed `--depth` shallow subset (**G9** semantics). Host green alone is not product-verified.
4. `spec check` clean for this SoT.
5. Workspace `cargo metadata`: `arks-remote` depends inward only; zero `gix`/gitoxide; zero `celir` edges.
