---
id: sot:root/arks-server
title: arks-server unit
status: active
created: 2026-11-08T23:12:49Z
updated: 2026-11-10T19:40:00Z
tags: [m11, m25, unit, server, ring-6, phase-sv, phase-sv2, smart-http, dumb-http, webdav, upload-pack, receive-pack, protocol-v2, git-daemon, ssh, production-server, githooks, http-basic, g036-http-basic-auth, r2, s3, self-host]
verify: manual
rel:
  governs: [crates/arks-server]
  depends_on:
    - sot:root/program-roadmap
    - plan:root/20261108105446
    - plan:root/20261109220230
    - adr:root/20261109220230
    - adr:root/20261109224236
    - adr:root/20261110190000
    - adr:root/20261110112600
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
    - sot:root/lease-concurrency-intent
    - sot:root/production-residual-register
    - research:root/20261108071916
    - research:root/20261108071859
    - adr:root/20261108083507
    - evidence:root/20261108150000
  evidence: [evidence:root/20261108231300, evidence:root/20261110200000, evidence:root/20261109224717, evidence:root/20261110190000, evidence:root/20261110112600]
---

# arks-server unit

Version **0.3.2**. Unit SoT for Ring 6 Git **server** (`crates/arks-server`) under [[plan:root/20261108105446]] / Phase **Sv** (**M11** — **production-server**) and [[plan:root/20261109220230]] / Phase **Sv2** (**M25** — transport expansion) + additive **R2** WebDAV write for dumb HTTP push + **S3** ref `DELETE`. Wire: [gitprotocol-http](https://git-scm.com/docs/gitprotocol-http), [gitprotocol-pack](https://git-scm.com/docs/gitprotocol-pack), [gitprotocol-v2](https://git-scm.com/docs/gitprotocol-v2), [git-upload-pack](https://git-scm.com/docs/git-upload-pack), [git-receive-pack](https://git-scm.com/docs/git-receive-pack), [git-http-backend](https://git-scm.com/docs/git-http-backend), [git-daemon](https://git-scm.com/docs/git-daemon), [git-http-push](https://git-scm.com/docs/git-http-push), [RFC 4918](https://datatracker.ietf.org/doc/html/rfc4918), [githooks](https://git-scm.com/docs/githooks). Catalog: [[research:root/20261108071916]]. Scope: [[sot:root/remote-host-compatibility-surface]]. Unlock: [[adr:root/20261109220230]]. Optional HTTP Basic: [[adr:root/20261109224236]]. Dumb push write: [[adr:root/20261110190000]]. Dumb push deletes: [[adr:root/20261110112600]].

**Entry:** M9 exit by default — [[evidence:root/20261108150000]]. **M10 is not a prerequisite.** Lease is not required for server.

**M11 does not redefine M9.** Production-server is a separate bar. **M25 does not redefine M11** — smart HTTP G15 remains the frozen production-server smart-HTTP receipt.

**G15 exit (frozen):** [[evidence:root/20261108231300]] — smart HTTP Claimed; matrix `verify/matrices/matrix-m011-clean-room-git-server.md`.

**M25 gates:** **G29** dumb HTTP; **G30** SSH command-mode; **G31** protocol v2; **G32** `git://` daemon — each Claimed only with its Docker receipt.

**Product self-host story** (localhost / LAN / WAN using only `arks`, bind vs ops TLS): [[sot:root/arks-self-host-server]] — design intent for hosting e2e; does not rewrite G15/G29–G32 wire receipts.

## Rule

### Role

1. **Git server only (A).** upload-pack / receive-pack over claimed transports so **stock Git** can clone/fetch/push. No forge UX / scope (B). Product **self-host reachability** narrative: [[sot:root/arks-self-host-server]].
2. **Smart HTTP required.** Production-server MUST claim smart HTTP (`info/refs?service=…` + `POST …/git-upload-pack|git-receive-pack`). Dumb-HTTP-only is **forbidden** as the sole claimed path; dumb HTTP MAY be claimed **additively** beside smart HTTP (M25 / G29).
3. **Protocol v1 baseline; v2 when claimed.** Advertise/negotiate v1 pack protocol for the claimed subset. Protocol v2 (`ls-refs` + `fetch` subset) MAY be advertised only when implemented and gated (M25 / G31) — never invent caps.
4. **Inward deps.** MAY depend on `arks-protocol`, `arks-pack`, Rings 0–4. MUST NOT depend on `arks-remote`, `arks-lease`, CELIR, or `gix`. MUST NOT depend on `arks` CLI types (CLI composes this crate).
5. **Filesystem `.git`.** Serve real repos via `arks-repo` / store / refs; honor L0 Git locks on receive-pack ref updates.

### Transport claim matrix

| Transport | Status | Gate / notes |
|-----------|--------|--------------|
| Smart HTTP | **Claimed (M11)** | G15 — required production-server path |
| Dumb HTTP (additive) | **Claimable → Claimed on G29** | [gitprotocol-http](https://git-scm.com/docs/gitprotocol-http) dumb; never sole path |
| SSH command-mode | **Claimable → Claimed on G30** | duplex gitprotocol-pack via `upload-pack` / `receive-pack` stdio |
| Protocol v2 | **Claimable → Claimed on G31** | [gitprotocol-v2](https://git-scm.com/docs/gitprotocol-v2) `ls-refs` + `fetch` subset |
| `git://` daemon | **Claimable → Claimed on G32** | [git-daemon](https://git-scm.com/docs/git-daemon); `arks daemon` |

### Refuse / honesty matrix (post-M25 unlock)

| Surface | Status | Product behavior |
|---------|--------|------------------|
| Product auth **server** (forge OAuth, GitHub App as server) | **Refuse-close / not claimed** | arks is not a forge auth product |
| Optional HTTP Basic (in-process) | **Claimed (G36)** | When configured (--basic-auth / ARKS_HTTP_BASIC / credential file); unset = unauthenticated loopback (G15); [[adr:root/20261109224236]], [[evidence:root/20261109224717]] |
| Ops reverse-proxy / external TLS | **Still valid ops pattern** | MAY front arks; additional to in-process Basic |
| Product SSHD / key CA | **Not claimed** | Operators wrap ForceCommand / `git-shell`; arks speaks pack protocol on stdio |
| `delete-refs` / `report-status-v2` | **Claimed** | Advertise + implement; classic ok/ng; FS2 may apply proc-receive rewrite options when diverted |
| Server-side [githooks](https://git-scm.com/docs/githooks) on receive-pack | **Claimed (honor list)** — see §Receive-pack githooks | Sync honor of `pre-receive`, `update`, `post-receive`, `post-update`, **`proc-receive` (FS2)**; residual **FW-SRV-HOOKS** |
| `push-options` (advertise + hook env) | **Claimed (O4)** | Advertise; parse `push-option` lines; set `GIT_PUSH_OPTION_*` for honor-list hooks; HOOK-008 race still **N/A** (FS8) |
| `proc-receive` | **Claimed (FS2)** | Advertise when `hooks/proc-receive` runnable; divert refs matching `receive.procReceiveRefs`; pkt-line version=0 + ok/ng + option refname/old-oid/new-oid rewrite subset; ≠ HOOK-008 race Claimed |
| Quarantine object dir (`GIT_QUARANTINE_PATH`) | **Claimed** | Incoming objects land under `$GIT_DIR/objects/incoming-*` and migrate only after `pre-receive` succeeds ([git-receive-pack](https://git-scm.com/docs/git-receive-pack#_quarantine_environment)) |
| Dumb-HTTP-only as sole server | **Forbidden** | Smart HTTP remains required |

### Receive-pack githooks (MUST — Claimed honor list)

Primary docs: [githooks](https://git-scm.com/docs/githooks), [git-receive-pack](https://git-scm.com/docs/git-receive-pack). Sync process only — **no** invented async / out-of-band hook runners. Isolate per-push hook stdin; do not share scratch across concurrent pushes. During unpack/`pre-receive`, set `GIT_QUARANTINE_PATH` to the incoming objects directory.

| Hook | When | Failure semantics |
|------|------|-------------------|
| `pre-receive` | Once, after pack unpack into quarantine and **before** migrate/ref update (and before per-ref fast-forward checks) | Non-zero exit → **reject entire push**: discard quarantine; no ref updates; no `update` / `post-receive` / `post-update`. report-status `ng <ref> pre-receive hook declined` for each proposed ref |
| `update` | Once per proposed ref, before that ref is updated | Non-zero exit → **reject that ref only** (`ng <ref> hook declined`); other refs MAY still update |
| `post-receive` | Once after processing, **if** at least one ref updated successfully | Exit status does **not** change push outcome |
| `post-update` | Once after processing, **if** at least one ref updated successfully; argv = updated ref names | Exit status ignored; typical use `update-server-info` for dumb HTTP |

Hooks run only when `$GIT_DIR/hooks/<name>` exists and is executable (Unix mode bits; Windows host advisory may invoke via `bash`). Ignore `*.sample`. Stdin for `pre-receive` / `post-receive`: one `old-oid SP new-oid SP refname LF` line per ref ([githooks](https://git-scm.com/docs/githooks)). `update` argv: `refname old-oid new-oid`. Set `GIT_DIR` to the served repository. Missing hook file = no-op (stock Git).

### Operations (MUST)

6. **upload-pack (v1):** Advertise refs + claimed caps; accept want/have/done; emit pack (undeltified OK) optionally side-band-64k when negotiated. HTTP ads include `# service=git-upload-pack`; SSH/`git://` duplex ads omit the HTTP service pkt-line ([gitprotocol-pack](https://git-scm.com/docs/gitprotocol-pack) vs [gitprotocol-http](https://git-scm.com/docs/gitprotocol-http)).
7. **receive-pack (v1):** Advertise refs + `report-status` (v1 only unless amended); accept update commands + pack; unpack via `arks-pack`; honor §Receive-pack githooks; update surviving refs under Git locks; emit report-status.
8. **Dumb HTTP:** On `GET …/info/refs` **without** `service=`, return dumb `info/refs` text (not `application/x-git-*`). Serve `objects/info/packs`, loose objects, and pack/idx files. `update-server-info` MUST refresh dumb advertisements.
8a. **Dumb HTTP push (WebDAV write, additive; R2 + S3):** Accept `PROPFIND` / `LOCK` / `UNLOCK` / `MKCOL` / `PUT` / `MOVE` / `DELETE` under `objects/`, `refs/`, and `info/` per [[adr:root/20261110190000]] + [[adr:root/20261110112600]]. `DELETE` is claimed for `refs/` only; MUST refuse deleting the current HEAD target. After successful ref PUT or DELETE, refresh `update-server-info`. Never claim dumb-only as the sole server path.
9. **Protocol v2 (when claimed):** On client `version=2` (HTTP `Git-Protocol` / git:// extra-param / `GIT_PROTOCOL`), advertise `version 2` + claimed keys only; execute `ls-refs`, `fetch`, and **`bundle-uri`** (G3 / NET-006 server) per [gitprotocol-v2](https://git-scm.com/docs/gitprotocol-v2). `bundle-uri` MUST advertise the capability; the command MUST emit `bundle.version=1` (+ optional `bundle.<id>.uri` from `ARKS_BUNDLE_URI` / `$GIT_DIR/arks-bundle-uris`). CDN SLOs are not Claimed. Stateless HTTP responses end with response-end-pkt where required.
9a. **LFS locks (G4):** Beside batch/basic, `serve-http` MAY honor `GET|POST …/info/lfs/locks`, `POST …/locks/verify`, and `POST …/locks/{id}/unlock` with `$GIT_DIR/lfs/locks.json` storage. tus / multi-tenant S3 custom transfer agents remain handoff/Partial.
10. **`git://` daemon:** Parse initial service request pkt-line (`git-upload-pack` / `git-receive-pack` + path + `host=`); require `git-daemon-export-ok` unless `--export-all`; then duplex pack protocol.
11. **Wave Sv / Sv2 CLI:** `arks upload-pack`, `arks receive-pack`, `arks http-backend`, `arks serve-http`, `arks daemon` (when G32 claimed), `arks update-server-info` (dumb). Help lists only implemented commands.
12. **Capability honesty.** Never advertise unimplemented caps. Refuse unknown services clearly.
13. **Auth.** Clean-room MAY be unauthenticated loopback when Basic is unset (G15). When Basic is configured, HTTP Git paths MUST challenge with 401 + WWW-Authenticate: Basic realm="arks" and accept RFC 7617 Basic credentials (summarize: Base64 of userid:password). Forge OAuth refused. Cite [[adr:root/20261109224236]].
14. **Residual honesty.** Help, CLI inventory, and public copy MUST match Claimed gates; unlocked-but-unverified surfaces MUST NOT be marketed as Claimed.

### Out of scope

- Forge (B); CELIR ObjectStore; requiring leases; inventing protocol caps; product credential / OAuth server.
- Folding into M9; calling M25 “CLI-parity production.”
- Lease-over-wire (until lease SoT claims it).
- HOOK-008 concurrent proc-receive / push-options race Claimed (catalog N/A retained until FS8).
- Negative `!` `receive.procReceiveRefs` forms (positive prefixes Claimed FS2).

## Why

Production-server (M11) makes arks a Git peer for stock clients over smart HTTP. M25 expands transports that Git documents for servers (SSH command-mode, dumb HTTP, protocol v2, `git://`) under the same honesty + clean-room rules.

## Verification

1. Host (advisory): `cargo test -p arks-server` (+ Wave Sv/Sv2 CLI tests via `-p arks`).
2. Clean-room **G15** smart HTTP — frozen Claimed receipt [[evidence:root/20261108231300]]; githooks honor list additive row **M11-C14** + [[evidence:root/20261110200000]].
3. Clean-room **G29–G32** — each transport Claimed only with its receipt; sole-runner Docker.
4. `spec check` clean; zero `gix` / zero `celir` edges on `arks-server`; public narrative matches Claimed gates only.
