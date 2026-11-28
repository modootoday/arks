---
id: research:root/20261128092500
title: v0.2.0 scope expansion implementation references and algorithms
status: active
created: 2026-11-28T09:25:00Z
updated: 2026-11-28T09:25:00Z
tags: [research, v0-2-0, reftable, git-replay, git-lfs, oauth-device-flow, github-actions, maintenance, subtree, self-host-ui]
verify: manual
rel:
  depends_on:
    - sot:root/product-intent
    - sot:root/git-compat-surface
    - sot:root/production-residual-register
    - sot:root/arks-hub-product-intent
    - sot:root/arks-self-host-server
    - research:root/20261111035700
  evidence: []
---

# v0.2.0 scope expansion — implementation references and algorithms

Primary-source technical research supporting [[sot:root/v0-2-0-scope-expansion]] and
[[plan:root/20261128092500]]. Every feature surveyed here was previously deferred, trimmed
([[research:root/20261111035700]]), or marked `Partial` / `OUT` in v0.1.x. **Honesty:** all
surfaces surveyed below are **Design / Research** only — none are Claimed or Verified until
implementation and Docker clean-room receipts land.

---

## 1. OAuth 2.0 Device Authorization Grant (`auth login --web`)

- **Primary sources:**
  - IETF RFC 8628 — *OAuth 2.0 Device Authorization Grant* (`https://www.rfc-editor.org/info/rfc8628/`)
  - GitHub Docs — *Authorizing OAuth Apps / Device Flow* (`https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps#device-flow`)
- **Why this replaces embedded browser callbacks:** No localhost HTTP callback listener, no
  browser-spawn requirement, works identically over SSH, headless VMs, and cloud agent sandboxes.
- **Wire protocol & state machine (RFC 8628 §3.1–§3.5):**
  1. **Device Authorization Request:** `POST <device_authorization_endpoint>` over HTTPS (`Accept: application/json`) with `client_id` and `scope`.
  2. **Response fields:** `device_code` (high-entropy secret, never printed), `user_code` (short human code, e.g. `XXXX-XXXX` using RFC 8628 §6.1 unambiguous charset), `verification_uri` (and optional `verification_uri_complete`), `expires_in` (seconds), `interval` (minimum polling interval, default 5s).
  3. **User prompt:** Print `verification_uri` and `user_code` to stderr/TTY with explicit phishing notice (RFC 8628 §5.4: confirm the code matches this CLI session).
  4. **Token Polling Loop:** `POST <token_endpoint>` with `grant_type=urn:ietf:params:oauth:grant-type:device_code`, `device_code`, `client_id`:
     - `authorization_pending` -> sleep `interval`, continue.
     - `slow_down` -> permanently increment `interval += 5s` for all subsequent polls (RFC 8628 §3.5), sleep new `interval`, continue.
     - `expired_token` / `access_denied` / deadline exceeded -> fail closed (non-zero), discard codes.
     - `200 OK` with `access_token` -> store via existing credential store / OS wallet (`arks-hub` auth store), never print raw token to stdout unless explicitly requested.
- **Security invariants:** HTTPS-only in production (plain HTTP allowed only for explicit local test fixtures); never log `device_code` or `access_token`.

---

## 2. GitHub Actions REST API & Client-Side Secret Sealing (`run`, `workflow`, `cache`, `secret`, `variable`)

- **Primary sources:**
  - GitHub REST API — *Actions* (`https://docs.github.com/en/rest/actions`)
  - GitHub REST API — *Actions Secrets* (`https://docs.github.com/en/rest/actions/secrets`)
  - NaCl / Libsodium Sealed Boxes (`https://docs.rs/crypto_box`, `https://libsodium.gitbook.io/doc/public-key_cryptography/sealed_boxes`)
- **Command groups promoted from `OUT` to optional GitHub-compatible extension:**
  - `workflow`: `list`, `view`, `run` (`POST .../dispatches`), `enable`, `disable`.
  - `run`: `list`, `view`, `watch` (polling with ETag / `If-None-Match` backoff), `rerun` (`--failed`), `cancel`, `download` (artifact ZIP unpack with zip-slip path validation), `delete`.
  - `cache`: `list`, `delete`.
  - `variable`: `list`, `get`, `set`, `delete` (repo / org / environment scopes).
  - `secret`: `list`, `set`, `delete` (repo / org / environment scopes).
- **Libsodium `crypto_box_seal` algorithm (pure Rust, zero C libsodium dependency):**
  1. Fetch recipient's 32-byte X25519 public key (`key`, `key_id`) from `GET /repos/{owner}/{repo}/actions/secrets/public-key` (or org/env equivalent) and Base64-decode `pk_r`.
  2. Generate an ephemeral X25519 keypair `(sk_e, pk_e)` via OS CSPRNG.
  3. Derive the 24-byte nonce deterministically: `nonce = BLAKE2b-192(pk_e || pk_r)`.
  4. Encrypt plaintext secret with `crypto_box` (X25519 DH + HSalsa20 key derivation + XSalsa20Poly1305 AEAD) producing `ciphertext || mac` (16-byte Poly1305 tag).
  5. Wire payload = `Base64(pk_e || ciphertext || mac)` (overhead = `32 + 16 = 48` bytes = `CRYPTO_BOX_SEALBYTES`).
  6. Zeroize `sk_e` and plaintext buffers immediately; `PUT .../secrets/{secret_name}` with `{ "encrypted_value": ..., "key_id": ... }`.
- **Implementation reference:** Pure-Rust RustCrypto `crypto_box` (`seal` feature) or `dryoc::classic::crypto_box::crypto_box_seal` (both Apache-2.0/MIT compatible, no C FFI).

---

## 3. Worktree-Free History Replay (`arks replay`)

- **Primary source:**
  - Git Documentation — `git-replay(1)` (`https://git-scm.com/docs/git-replay`, introduced in Git 2.44 and extended through Git 2.54)
- **Why this is strategic for `arks`:** Multi-agent sessions contend heavily on working trees and
  `.git/index`. `replay` rebases commit ranges purely in the object store (in-memory tree merges)
  without touching the working tree or index, then emits atomic ref updates.
- **Core algorithm:**
  1. Parse revision range(s) (`A..B`, `--onto <newbase>`, `--advance <branch>`, `--contained`, `--revert`, `--ref-action=update|print`).
  2. Walk commits in reverse topological order (`--reverse --topo-order`).
  3. Maintain `commit_map: HashMap<Oid, Oid>` mapping original commit OIDs to replayed OIDs.
  4. For each commit `C` with parent `P` (or empty tree for root commits):
     - Resolve target base commit `B_new = commit_map.get(P).unwrap_or(onto)`.
     - Compute 3-way tree merge in memory: `merge_trees(base = tree(P), ours = tree(B_new), theirs = tree(C))` using `arks-repo`'s tree merge engine (content-level 3-way blob merge, mode/rename conflict detection, no worktree writes).
     - On conflict -> fail closed with exit code `1` (no ref mutation, no partial worktree state).
     - Drop commits that become empty (`tree(new) == tree(B_new)`) when original was non-empty (matching Git 2.54 behavior), unless explicitly preserved.
     - Write new commit object preserving author, message, and encoding, with updated parent(s) and committer timestamp; record `commit_map[C] = C_new`.
  5. **Ref update phase:**
     - Default (`--ref-action=update`): apply all affected branch updates in a single atomic ref transaction (all succeed or none mutate).
     - `--ref-action=print`: emit `update <refname> <new-oid> <old-oid>` lines to stdout compatible with `arks update-ref --stdin`.

---

## 4. Repository Maintenance & Subtree (`arks maintenance`, `arks subtree`)

- **Primary sources:**
  - Git Documentation — `git-maintenance(1)` (`https://git-scm.com/docs/git-maintenance`)
  - Git `contrib/subtree` (`https://github.com/git/git/tree/master/contrib/subtree`)
- **`arks maintenance` algorithm & tasks:**
  - Subcommands: `register`, `unregister`, `start`, `stop`, `run [--auto] [--schedule=<freq>] [--task=<t>]`.
  - Strategies (`maintenance.strategy`): `none`, `gc`, `incremental` (`prefetch` hourly, `commit-graph` hourly, `loose-objects` daily, `incremental-repack` daily, `pack-refs` weekly), `geometric` (geometric packfile repacking + cruft pack for unreachable objects).
  - Race-safe `incremental-repack`: two-step MIDX protocol — (1) `multi-pack-index expire` deletes unreferenced packs, (2) `multi-pack-index repack --batch-size=<N>` consolidates small packs into a larger pack and atomically updates MIDX before next expire.
  - Background scheduling (`start`/`stop`): OS-native scheduler adapters (`systemd --user` timers / `crontab` on Linux, `launchd` on macOS, Task Scheduler on Windows) with `--scheduler=auto|crontab|systemd-timer|launchd|schtasks` and explicit dry-run/lockfile (`maintenance.lock`) coordination.
- **`arks subtree` algorithm:**
  - Subcommands: `add`, `pull`, `push`, `merge`, `split`.
  - Trailers in merge/squash commits: `git-subtree-dir: <prefix>`, `git-subtree-mainline: <oid>`, `git-subtree-split: <oid>`.
  - `split --prefix=<P>` algorithm: walk commit DAG in topological order, project each commit's tree to subtree at `<P>`, skip commits where projected tree equals parent's projected tree, memoize `orig_oid -> split_oid` (persisted cache for incremental re-splits), and reconstruct synthetic commits preserving parent topology.

---

## 5. Git Reftable Storage Backend (`arks-refs` + `arks refs migrate`)

- **Primary sources:**
  - Git Technical Documentation — *Reftable Format* (`https://git-scm.com/docs/reftable`, `https://www.kernel.org/pub/software/scm/git/docs/technical/reftable.html`)
  - Git Documentation — `git-refs(1)` (`https://git-scm.com/docs/git-refs`)
- **Why this is strategic for `arks`:** Eliminates global `packed-refs.lock` contention and
  directory-file (`D/F`) filesystem races when hundreds of agent sessions create/delete refs
  concurrently, while remaining 100% standard Git (`.git/reftable/`, `extensions.refStorage = reftable`).
- **On-disk binary format (standard Git v1/v2):**
  - Config activation: `[core] repositoryformatversion = 1` + `[extensions] refStorage = reftable`.
  - Backward-compatibility stubs: `$GIT_DIR/HEAD` contains `ref: refs/heads/.invalid` and `$GIT_DIR/refs/heads` is a stub file so legacy tools recognize `$GIT_DIR` without corrupting refs.
  - **File header (24 bytes v1 / 28 bytes v2 with hash ID `sha1` or `s256`):** magic `REFT`, version byte (`1` or `2`), 3-byte `block_size` (default 4096 or 65536), 8-byte `min_update_index`, 8-byte `max_update_index`.
  - **Block types:**
    - `'r'` (ref block): `block_type` (1B) + `block_len` (3B) + prefix-compressed `ref_record` stream + sorted 3-byte `restart_offset` array (every 16 records for 4KiB blocks, 64 for 64KiB; first block offset starts at 28 including header) + 2-byte `restart_count`.
    - `'i'` (index block): binary-searchable last-key -> block-offset index for single-seek lookup.
    - `'o'` (obj block): optional reverse OID-prefix -> ref-block-offset index.
    - `'g'` (log block): zlib-compressed reflog records grouped by `(refname, !update_index)` for backward reflog scans.
  - **`ref_record` value types (3-bit `value_type`):**
    - `0x0`: explicit deletion tombstone (no value payload).
    - `0x1`: direct OID (`hash_len` bytes).
    - `0x2`: direct OID + peeled tag target OID (`2 * hash_len` bytes).
    - `0x3`: symbolic ref (`varint(len) + target_bytes`).
  - **Footer (68 bytes v1 / 72 bytes v2):** copy of header + `ref_index_position`, `obj_position`, `obj_index_position`, `log_position`, `log_index_position`, trailing CRC-32 checksum.
- **Concurrency & compaction algorithm:**
  - Stack manifest: `$GIT_DIR/reftable/tables.list` listing immutable table filenames (`<min_idx>-<max_idx>-<random>.ref`) from oldest to newest.
  - **Lock-free consistent reads:** read `tables.list`, open referenced immutable `.ref` files, perform k-way merged iteration (`min-heap` by `(refname, -update_index)`). Because tables are immutable and uniquely named, readers never block writers (and Windows open-file handles never break compaction).
  - **Atomic writes:** acquire `tables.list.lock`, write new small `.ref` file, `fsync`, append filename to `tables.list.lock`, atomic rename over `tables.list`.
  - **Geometric compaction (factor = 2):** scan stack sizes `[s_0, s_1, ..., s_{n-1}]`; find highest index `k` where `s_{k-1} < 2 * sum(s_k..s_{n-1})`; merge-join tables `k..n-1` into one new table, dropping `0x0` tombstones only if compacting down to the bottom of the stack (`k == 0`); atomically swap entries in `tables.list` and unlink obsolete tables.
- **`arks refs migrate` & `arks refs verify`:** bidirectional lossless conversion (`files` <-> `reftable`, including reflogs unless `--no-reflog`, plus `--dry-run`) and structural/CRC/ordering verification.

---

## 6. Native Git LFS Client and Self-Host Server (`arks lfs` + `arks serve` `info/lfs`)

- **Primary sources:**
  - Git LFS Specification — *Pointer & Filter Protocol* (`https://github.com/git-lfs/git-lfs/blob/main/docs/spec.md`)
  - Git LFS API — *Batch API* (`https://github.com/git-lfs/git-lfs/blob/main/docs/api/batch.md`)
  - Git LFS API — *Basic Transfer API* (`https://github.com/git-lfs/git-lfs/blob/main/docs/api/basic-transfers.md`)
  - Git LFS API — *File Locking API* (`https://github.com/git-lfs/git-lfs/blob/main/docs/api/locking.md`)
- **Pointer v1 canonical format (UTF-8, LF-terminated, sorted keys, <= 1024 bytes):**
  ```text
  version https://git-lfs.github.com/spec/v1
  oid sha256:<64-hex-lowercase>
  size <non-negative-decimal-bytes>
  ```
- **Local object store:** `$GIT_DIR/lfs/objects/<oid[0..2]>/<oid[2..4]>/<oid>` with atomic temp-file write + streaming SHA-256 verification on both `clean` and download.
- **Filter integration:**
  - In-process `clean` and `smudge` filters for `.gitattributes` entries matching `filter=lfs` (plus CLI `arks lfs clean`, `arks lfs smudge`, `arks lfs filter-process` speaking Git's pkt-line long-running filter protocol `git-filter-client` / `git-filter-server` v2 so large checkouts avoid per-file process forks).
  - Pre-push scan: walk commits being pushed, detect pointer blobs, batch-upload missing OIDs before pushing Git packfile.
- **Batch API & Basic Transfer protocol:**
  - Endpoint discovery: `<remote_url>.git/info/lfs/objects/batch` (or explicit `lfs.url`).
  - Request headers: `Accept: application/vnd.git-lfs+json`, `Content-Type: application/vnd.git-lfs+json`.
  - `POST /info/lfs/objects/batch` with `{ "operation": "upload"|"download", "transfers": ["basic"], "objects": [{"oid": "...", "size": N}], "hash_algo": "sha256" }`.
  - Server responds with per-object `actions.upload` / `actions.download` / optional `actions.verify` URLs and headers; client streams PUT/GET with SHA-256 verification and bounded concurrency.
- **Server-side `arks serve` (`info/lfs`):**
  - Opt-in/enabled alongside HTTP Git wire in `arks-server`: `POST /<repo>.git/info/lfs/objects/batch`, `PUT /<repo>.git/info/lfs/objects/<oid>`, `GET /<repo>.git/info/lfs/objects/<oid>`, `POST /<repo>.git/info/lfs/objects/verify`, reusing `arks-server` Basic/Bearer auth and repo permissions.

---

## 7. Single-Binary Embedded Web UI for `arks serve`

- **Design constraints:**
  - Must preserve the single-binary invariant ([[sot:root/product-naming]]): zero Node.js runtime, zero external asset directory required at runtime; CSS/HTML templates embedded via `include_str!` / compile-time Rust templates.
  - Strict security defaults: HTML auto-escaping on all user/repo strings, `Content-Security-Policy: default-src 'self'; script-src 'none'` (zero inline JS required for v1 read/review SSR), CSRF double-submit or SameSite token on any future POST form.
  - Opt-out flag `--no-web-ui` on `arks serve` (mirrors `--no-http`, `--no-ssh`, `--no-forge`).
- **v1 scope (human code-review ergonomics for self-host teams):**
  - Repository index, branch/tag selector, tree & blob viewer (line numbers, raw download), commit log & unified diff viewer, Pull Request list/detail/diff/comment view, Issue list/detail/comment view — all backed directly by existing `arks-repo` and `arks-server` forge storage.

---

## 8. Explicit Keep-Refuse Boundaries (Unchanged)

The following remain **out of scope / Refuse** in `v0.2.0`:
- Embedded desktop GUIs (`gitk`, `git-gui`, `citool`, `gui.tcl`).
- GitHub-proprietary ecosystem commands (`codespace`, `copilot`, `extension`, `agent-task`, `event*`, `skill`).
- Cloud-sync folder safety (`arks host-env` for OneDrive/Dropbox/iCloud).
- CELIR alternative object store (permanently withdrawn per [[adr:root/20261119200000]]).
- Absolute SHA-1 collision immunity (detect+refuse remains the ceiling) and classic UNIX double-fork daemonization.
