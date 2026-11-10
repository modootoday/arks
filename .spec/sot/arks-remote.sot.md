---
id: sot:root/arks-remote
title: arks-remote unit
status: active
created: 2026-11-08T11:05:00Z
updated: 2026-11-10T11:26:00Z
tags: [m6, m16, unit, remote, client, smart-http, dumb-http, webdav, ssh, git-daemon, ring-6, phase-h, local-clone, g9, g35, shallow, r1, r2, r3, filter, promisor, s1, s3]
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
    - adr:root/20261110190000
    - adr:root/20261110105547
    - adr:root/20261110112600
    - research:root/20261110105436
    - evidence:root/20261108104837
  evidence: [evidence:root/20261108112000, evidence:root/20261110180000, evidence:root/20261110190000, evidence:root/20261110105855, evidence:root/20261110112600]
---

# arks-remote unit

Version **0.5.4**. Unit SoT for Ring 6 Git **remote client** (`crates/arks-remote`) under [[plan:root/20261108090651]] / roadmap Phase **H** (**M6**) + additive **G9/G35** shallow + dumb HTTP fetch/clone fallback + **`git://` client (R1)** + **dumb HTTP push (R2)** + **partial-clone `blob:none` (R3 / S1)** + **dumb HTTP push deletes (S3)**. Wire bytes cite [gitprotocol-http](https://git-scm.com/docs/gitprotocol-http), [gitprotocol-pack](https://git-scm.com/docs/gitprotocol-pack), [gitprotocol-common](https://git-scm.com/docs/gitprotocol-common), [gitprotocol-capabilities](https://git-scm.com/docs/gitprotocol-capabilities) (`shallow` / `deepen` / `filter` / `delete-refs`), [partial-clone](https://git-scm.com/docs/partial-clone), [git-daemon](https://git-scm.com/docs/git-daemon), [git-http-push](https://git-scm.com/docs/git-http-push), [RFC 4918](https://datatracker.ietf.org/doc/html/rfc4918); catalog [[research:root/20261108071916]] + [[research:root/20261108071917]]. Host verify-peer policy: [[sot:root/remote-host-compatibility-surface]]. Historical ladder: [[research:root/20261110095142]]. Symmetry ladder: [[research:root/20261110105436]]. Unlock R2: [[adr:root/20261110190000]]. Unlock R3: [[adr:root/20261110105547]]. Unlock S3 deletes: [[adr:root/20261110112600]].

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
| Dumb HTTP fetch/clone | **Claimed (additive)** | Fall back when smart `info/refs` returns dumb body ([gitprotocol-http](https://git-scm.com/docs/gitprotocol-http#_dumb_clients)); packs via `objects/info/packs` + loose OID GETs; `--depth` refused on dumb |
| Dumb HTTP push | **Claimed (additive; R2 + S3 deletes)** | When receive-pack discovery is dumb, WebDAV write ([git-http-push](https://git-scm.com/docs/git-http-push), [RFC 4918](https://datatracker.ietf.org/doc/html/rfc4918)): PROPFIND/LOCK/MKCOL/PUT/MOVE/DELETE/UNLOCK; FF-only; zero-oid / `:<ref>` deletes; refuse deleting remote HEAD target; host wire vs `arks serve-http --dumb-only` [[evidence:root/20261110190000]] + [[evidence:root/20261110112600]] |
| `git://` | **Claimed (additive; R1)** | TCP pack-protocol client to git-daemon peers ([git-daemon](https://git-scm.com/docs/git-daemon), [gitprotocol-pack](https://git-scm.com/docs/gitprotocol-pack)); clone/fetch (`git-upload-pack`) + push (`git-receive-pack` when peer enables receive); default port **9418**; host wire roundtrip vs `arks daemon` |

### Operations (MUST for claimed subset)

5. **Remote config.** Read/write `[remote "<name>"]` `url` and `fetch` refspec lines in `.git/config` as Git documents for the claimed subset (add / list / remove).
6. **Fetch / clone path (HTTP).** `GET …/info/refs?service=git-upload-pack` then `POST …/git-upload-pack`; parse v1 advertisement; send want/have/done; demux side-band when negotiated; decode pack via `arks-pack`; **ingest as pack+idx** (prefer over exploding every object to loose); update remote-tracking refs per fetch refspec.
6a. **Fetch / clone path (SSH).** OpenSSH `ssh` (or `GIT_SSH_COMMAND` / `ARKS_SSH_COMMAND`) runs peer `git-upload-pack '<path>'`; speak gitprotocol-pack v1 advertisement + want/have/done over the duplex; demux side-band when negotiated; ingest pack+idx; update tracking refs. Cite [[research:root/20261108071916]] SSH/pack note.
6b. **Fetch / clone path (`git://`).** TCP connect to `host:port` (default **9418**); send initial service pkt-line `git-upload-pack /path\0host=…\0`; then same pack-protocol negotiation/ingest as SSH. Cite [git-daemon](https://git-scm.com/docs/git-daemon) + [[research:root/20261108071916]].
7. **Push path (HTTP).** `GET …/info/refs?service=git-receive-pack` then `POST …/git-receive-pack`; send update commands + pack; parse `report-status` / `report-status-v2` as advertised for the claimed subset. When the advertisement is dumb (non-`application/x-git-*`), fall back to **dumb HTTP push** (rule 7c).
7c. **Push path (dumb HTTP / WebDAV).** LOCK target ref; MKCOL object fanout; PUT zlib loose objects (temp name + MOVE); PUT ref `oid LF`; UNLOCK; refuse non-fast-forward. Zero `new_oid` / CLI `:<ref>`: LOCK + `DELETE` + UNLOCK; MUST refuse deleting the remote HEAD symref target. Cite [[adr:root/20261110190000]] + [[adr:root/20261110112600]].
7a. **Push path (SSH).** Same pack-protocol push dialog over OpenSSH running peer `git-receive-pack '<path>'`.
7b. **Push path (`git://`).** Same receive-pack dialog after service request `git-receive-pack /path\0host=…\0`; peer MUST enable receive-pack (arks: `--enable=receive-pack`). Half-close write after the push body so the peer can finish `report-status`.
8. **Clone composition (HTTP).** Init repo layout + fetch + baseline checkout of HEAD tree into the work tree (and index) for the claimed non-bare case.
8a. **Clone composition (local path / `file://`).** Resolve path; discover source `$GIT_DIR`; init dest layout; hardlink-or-copy `objects/` from source (prefer hardlink; packs kept; pack-aware store); copy heads/tags + set `HEAD` / `origin` tracking; baseline checkout. Refuse non-local `file://` hosts and non-path schemes outside this table.
8b. **Clone composition (SSH).** Init + SSH fetch + baseline checkout (same as HTTP composition after objects arrive).
8c. **Clone composition (`git://`).** Init + `git://` fetch + baseline checkout (same as HTTP composition after objects arrive).
9. **Capability honesty.** Prefer protocol v1 for M6 product claim (v2 MAY be parsed later). Never advertise unimplemented caps on the client want line beyond the negotiated subset. Unknown optional peer caps MAY be ignored.
10. **Auth.** HTTPS: optional credentials via URL userinfo; `ARKS_REMOTE_USERNAME`/`ARKS_REMOTE_PASSWORD`; configured `credential.helper` (including first-party `wincred` / `libsecret` / `osxkeychain`); `GIT_ASKPASS`/`core.askPass`/`SSH_ASKPASS`; and TTY terminal prompt when `GIT_TERMINAL_PROMPT`≠`0` — [[sot:root/credential-helper-askpass]]. No forge OAuth UX. SSH: OpenSSH agent / key files / `GIT_SSH_COMMAND`. Secrets MUST NOT enter the tree or fixtures. Recorded/local peers MAY need no auth.
11. **Pull.** Incomplete `pull` (fetch+merge without Pa merge) MUST refuse non-zero without mutating remote state. SSH URLs are claimed under Sy-4 (not refuse).

### Claimed shallow subset (G9 / clean-room **G35**; v0.4.0)

12. **`--depth N` only.** Client MAY request positive commit depth on smart HTTP, SSH, and `git://` clone/fetch when the peer advertises the `shallow` capability ([gitprotocol-capabilities](https://git-scm.com/docs/gitprotocol-capabilities) `shallow`; [git-clone `--depth`](https://git-scm.com/docs/git-clone#Documentation/git-clone.txt---depthltdepthgt)). Wire: request `shallow` on the first want line; send `deepen N`; parse server `shallow` / `unshallow` lines; write `$GIT_DIR/shallow` (gitrepository-layout). Existing local shallow OIDs MUST be re-sent as client `shallow` lines on later fetches.
13. **Capability honesty.** MUST NOT request `shallow` / `deepen` / `filter` unless implemented and peer-advertised. Depth against a peer that does not advertise `shallow` MUST refuse. Local path / `file://` clone MUST refuse `--depth` (no pack-protocol deepen on the claimed local object-dir path).
14. **Depth non-claims (remain out).** `deepen-since` / `deepen-not` / `deepen-relative`, `--unshallow`, server-side shallow product. Clean-room proof for depth: [[sot:root/interop-test-matrix]] **G9** via compose **G35** (`verify/matrices/matrix-g35.md`).

### Claimed partial-clone subset (R3 / symmetry **S1**; v0.5.3)

15. **`--filter=blob:none` only.** Client MAY request `blob:none` on smart HTTP, SSH, and `git://` clone/fetch when the peer advertises `filter` ([partial-clone](https://git-scm.com/docs/partial-clone); [gitprotocol-pack](https://git-scm.com/docs/gitprotocol-pack) `filter-request`; [git-clone `--filter`](https://git-scm.com/docs/git-clone#Documentation/git-clone.txt---filterltfilter-specgt)). Wire: request `filter` on the first want line; send `filter blob:none` before flush. Other filter-specs MUST refuse fail-closed.
16. **Promisor + extension.** After a successful filtered pack ingest, MUST set `core.repositoryformatversion=1` and `extensions.partialClone=<remote-name>`, and MUST mark the pack with a sibling `.promisor` file. Demand-fetch packs that backfill missing objects for checkout MUST also be marked promisor.
17. **Checkout backfill.** Clone composition MUST demand-fetch missing blobs required for the HEAD tree from the promisor remote before completing worktree materialization (user-visible parity with stock `git clone --filter=blob:none`). Historical blobs MAY remain absent.
18. **Partial-clone non-claims.** `tree:0`, `blob:limit=*`, sparse OID filters, multi-promisor ordering product, local/`file://` `--filter`, server-side filter as a sole server product. Unlock: [[adr:root/20261110105547]].

### Out of scope (M6)

- Git server role / `arks-server` (**M11**).
- Forge product APIs (B).
- CELIR; lease L1+ runtime (**M10**).
- Filter-specs beyond the `blob:none` subset above.
- Calling M6 **production**.
- Permanent `gix`; product-path `git` subprocess (stock Git allowed only as clean-room oracle / local peer fixture).
- `--shared` / alternates local clone; piped `file://` upload-pack clone (may amend later). Hardlink-prefer for object files is claimed (v0.4.0).
- Redefining **M9** / **M10** / **M11**.

## Why

Wave H unlocks remotes before porcelain waves. A Ring 6 client over proven pkt-line/pack libraries keeps wire semantics standards-first and unlocks clean-room **G7** without waiting for M11 server or forge APIs. Symmetry **S1** closes the honest-N/A filter/promisor gap without inventing non-Git wire.

## Verification

1. Host (advisory): `CARGO_BUILD_JOBS=2 cargo test -p arks-remote` and Wave H CLI tests via `-p arks`.
2. Clean-room **G7** `remote-recorded` (+ G1–G6 regression) per `verify/matrices/matrix-m6.md` — authoritative. **G8** default **Refuse-closed** without opt-in secrets ([[adr:root/20261110151000]]); live only when `ARKS_G8_LIVE=1` + token + URL. Host green alone is not product-verified.
3. Clean-room **G35** (`verify/matrices/matrix-g35.md`, compose `g35`) for the claimed `--depth` shallow subset (**G9** semantics). Host green alone is not product-verified.
4. Residual **R3** / symmetry **S1**: host wire (or clean-room twin) MUST prove `clone --filter=blob:none` vs stock peer with `filter` advertised → `extensions.partialClone` + `.promisor` + HEAD worktree; Claimed only with evidence. Unclaimed filter-specs MUST still refuse. Symmetry e2e expansion is **S6**.
5. `spec check` clean for this SoT.
6. Workspace `cargo metadata`: `arks-remote` depends inward only; zero `gix`/gitoxide; zero `celir` edges.
