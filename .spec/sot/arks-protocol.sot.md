---
id: sot:root/arks-protocol
title: arks-protocol unit
status: active
created: 2026-11-08T10:12:00Z
updated: 2026-11-08T10:12:00Z
tags: [m4, unit, protocol, pkt-line, ring-5, phase-p]
verify: manual
rel:
  governs: [crates/arks-protocol]
  depends_on:
    - sot:root/program-roadmap
    - plan:root/20261108090649
    - sot:root/git-compat-surface
    - sot:root/crate-architecture
    - sot:root/arks-pack
    - research:root/20261108071916
    - adr:root/20261108083507
    - evidence:root/20261108191033
  evidence: [evidence:root/20261108103610]
---

# arks-protocol unit

Version 0.1.0. Unit SoT for Ring 5 pkt-line + capability helpers ([[plan:root/20261108090649]], M4 Phase P under [[sot:root/program-roadmap]] **v0.3.1**). Framing from [gitprotocol-common](https://git-scm.com/docs/gitprotocol-common); capabilities from [gitprotocol-capabilities](https://git-scm.com/docs/gitprotocol-capabilities); v2 special packets from [gitprotocol-v2](https://git-scm.com/docs/gitprotocol-v2). Cite [[research:root/20261108071916]].

**Entry:** M3 exit proven — [[evidence:root/20261108191033]]. Libraries only — **no** remote porcelain (M6).

## Rule

1. **Public surface (M4 claimed):** pkt-line encode/decode; capability advertise/parse helpers; offline fixture builders/parsers sufficient for clean-room **G5**. Prefer modular `pkt_line` / `capabilities` / `error` layout.
2. **pkt-line (MUST):**
   - 4 hex length bytes (lowercase hex digits `0-9a-f`) including themselves; payload length = `pkt-len - 4`.
   - Max payload **65516** bytes; MUST refuse encode above that (total line ≤ 65520).
   - **flush-pkt** `0000` (special; not an empty data line).
   - Data pkt-lines MAY be binary (8-bit clean). Non-binary lines SHOULD include trailing LF when encoded for text helpers; receivers MUST accept with or without trailing LF per gitprotocol-common.
   - SHOULD NOT emit empty data pkt-line `0004` from helpers (document if a parse must accept it).
3. **v2 special packets (MUST parse/encode for fixtures):**
   - **delim-pkt** `0001` and **response-end-pkt** `0002` as documented in gitprotocol-v2 (length fields, not data payloads). M4 claims framing helpers only — not a full v2 fetch/push client.
4. **Capabilities (MUST):**
   - Parse space-separated capability tokens from a v1 first-ref / `capabilities^{}` style trailer string.
   - Build an advertise string from an ordered token list.
   - **Negotiate policy (M4):** prefer **negotiate readiness** — parse/advertise helpers MUST NOT invent capability names; only emit tokens the caller supplies. Unknown peer caps MAY be retained as opaque strings for later filtering. Do **not** claim live upload-pack/receive-pack sessions in M4.
   - Open ordering (v1-first vs v2-day-one) remains tracked on [[plan:root/20261108090649]]; M4 resolves library readiness for **both** framings without inventing wire semantics.
5. **Dependencies:** MAY depend on Rings 0–4 / `arks-pack` as needed for fixture helpers; MUST NOT depend on CLI (`arks` bin) or `arks-remote`.
6. **No `gix`** and no product-path `git` subprocess ([[adr:root/20261108083507]]).
7. **Out of scope (M4):** live HTTP/SSH remotes; GitHub-class client (M6); side-band demux product claims beyond offline fixture bytes if needed for G5; server role; porcelain.

## Why

Exact pkt-line and capability string handling is the prerequisite for every smart transport. Shipping Ring 5 helpers with offline **G5** fixtures proves wire framing before M5 CLI or M6 remotes exist.

## Verification

1. Host (advisory): `CARGO_BUILD_JOBS=2 cargo test -p arks-protocol`.
2. Clean-room **G5** `protocol-offline` (`verify/matrices/matrix-m004-clean-room-pack-protocol.md`, `verify/gates/g005-protocol-offline-fixtures`) — authoritative offline fixture replay; carrying receipt: [[evidence:root/20261108103610]]. Host green alone is not product-verified.
3. `spec check` clean for this SoT.
4. Workspace `cargo metadata` shows no `gix` / gitoxide edge on `arks-protocol`.
