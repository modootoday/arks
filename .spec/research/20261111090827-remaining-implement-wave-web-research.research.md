---
id: research:root/20261111090827
title: Remaining implement-shaped residuals — web research + failure modes
status: active
created: 2026-11-11T00:08:27Z
updated: 2026-11-11T00:08:27Z
tags: [research, residual, b5, b7, ustar, upload-archive, openssh, rebind, rustls, tls, cli-stubs, personality, forge-in-serve, no-invent-claimed, docker]
verify: manual
rel:
  depends_on:
    - research:root/20261111084400
    - research:root/20261111074523
    - research:root/20261111072342
    - sot:root/arks-claimed-residual-gates
    - sot:root/arks-remaining-implement-wave
    - sot:root/production-residual-register
    - sot:root/verify-directory-conventions
    - sot:root/arks-self-host-server
    - sot:root/arks-cli-personality-factory
    - sot:root/product-verification-clean-room
  related:
    - crates/arks-server/src/daemon.rs
    - verify/scripts/run-claimed-daemon-archive-e2e.sh
    - verify/scripts/run-claimed-rebind-e2e.sh
---

# Remaining implement-shaped residuals — web research + failure modes

English-only. Tip surveyed: **`8775cc4`**. **Design ≠ Claimed.** **Do not implement product code from this research alone** — cite this note + owning design SoT ([[sot:root/arks-remaining-implement-wave]]) before any crate change.

## Problem

Audit left four implementation-shaped residual classes open **by honesty**, not by missing Spec inventory:

1. **CLI Partial stubs** still open by design (hub flat-leaf depth, personality shell, forge-in-serve depth, daemon deepen, serve SSH **process-label** Partial).
2. **B5** byte-identical ustar vs stock Git **2.39.5** — prior wave closed as **Partial (terminal for that goal)**; reopen path exists ([[research:root/20261111084400]]).
3. **B7** SSH/forge **rebind** still **Partial** while HTTP rebind is **Claimed**.
4. Other residual-register “next” rows that are still implement-shaped (not Refuse/N/A).

Prior thrash burned tokens by coding toward Claimed **without** freezing normative encoder facts, Docker oracles, and Partial-vs-Claimed bars first.

## Sources (web / pin tree)

| Topic | URL | Fact used |
|-------|-----|-----------|
| Git **v2.39.5** `prepare_header` | https://raw.githubusercontent.com/git/git/v2.39.5/archive-tar.c | Numeric fields `"%07o"` / `"%011o"` via `xsnprintf`; **devmajor/devminor always written**; chksum `"%07o"` of space-substituted sum; magic `"ustar\0"` + version `"00"`; uname/gname `root` |
| Master `archive-tar.c` (context) | https://github.com/git/git/blob/master/archive-tar.c | Same prepare_header shape on current tree; **not** the Claimed pin — pin remains **2.39.5** |
| Chksum layout debate (GNU 6-digit + NUL + SP) | https://public-inbox.org/git/20190723025736.23036-1-mattst88@gmail.com/T/ | Proposed GNU-style chksum; **v2.39.5 still uses `"%07o"`** — Claimed MUST NOT invent GNU layout against the pin |
| `git-upload-archive` | https://git-scm.com/docs/git-upload-archive | Remote archive is `git archive --remote` ↔ `upload-archive` over Git protocol (daemon enables it) |
| Pack protocol SSH transport | https://git-scm.com/docs/pack-protocol | Git-over-SSH = remote **command** `git-upload-pack '/path'` (not OpenSSH `Subsystem` by default) |
| OpenSSH `sshd_config` | https://man.openbsd.org/OpenBSD-current/man/sshd_config | `Subsystem` names external programs (e.g. sftp); `ForceCommand` overrides client command — ops SSHD patterns ≠ in-process serve Claimed |
| Zero-downtime / dual-bind pitfalls | https://blog.cloudflare.com/ecdysis-rust-graceful-restarts/ ; https://github.blog/news-insights/glb-part-2-haproxy-zero-downtime-zero-delay-reloads-with-multibinder/ ; https://goteleport.com/blog/golang-ssh-bastion-graceful-restarts/ | `SO_REUSEPORT` dual-bind can orphan accept-queue SYNs; Claimed drain needs held peers + overlap evidence; FD inheritance is stronger for multi-process — arks same-process dual-bind still valid if receipt proves no hard-cut |
| rustls / tokio-rustls server | https://github.com/rustls/tokio-rustls/blob/main/examples/server.rs ; https://rustls.dev/docs/ | In-process terminate: `ServerConfig` + PEM + `TlsAcceptor` per accept |
| rustls vs native-tls | https://softwarepatternslexicon.com/rust/networking-and-i-o-patterns/network-security-and-tls/ ; https://rs4ts.dev/27-security/05-tls-ssl/ | Prefer **rustls** for product servers (no OpenSSL dep, cross-OS predictability); native-tls only for deliberate platform-stack needs |

Clean-room pin remains Git **2.39.5** ([[sot:root/product-verification-clean-room]], `verify/images.lock.json`).

## Failure modes — why prior thrash failed

| Mode | What happened | Prevention (this wave) |
|------|---------------|-------------------------|
| **Blind Docker retries** | B5 compose/scripts exit 1/101 repeated without freezing root cause | Stop after one classified mismatch; Spec-amend or fix once with unit+Docker golden |
| **Semantic ≠ byte-identical** | Python `tarfile` / member lists green while raw header bytes differ (501009: chksum@151, device fields) | Claimed oracle is **`cmp -s`** only; structural stays Partial |
| **Wrong encoder dialect** | Temptation to adopt GNU chksum (`%06o` + NUL + SP) or a Rust `tar` crate | Pin oracle is **v2.39.5 `prepare_header`**, not POSIX folklore or libarchive |
| **Host Git skew** | Host 2.56 dumps treated as authority | Judge = Docker pin only |
| **Invent Claimed from Partial** | Smoke / SKIP / live rebind without drain receipt marketed as Done | Partial smokes stay Partial; STRICT fail-closed |
| **B7 HTTP success over-read** | HTTP dual-bind Claimed used as if SSH/forge rebind graduated | Split B7 matrix: HTTP Claimed; SSH/forge Partial until separate receipts |
| **Process-label honesty drift** | Durable SSH listen still prints Partial while B1/B2 wire Claimed — operators confuse labels | Explicit **process-label Claimed** bar separate from wire/clone Claimed |
| **Coding before SoT** | Crate edits without acceptance slots → token burn | Gate: research + [[sot:root/arks-remaining-implement-wave]] cited in Spec-first commit before product |

## Options by residual class

### A — B5 ustar Claimed reopen

| Option | Summary | Verdict |
|--------|---------|---------|
| **A1 Match `prepare_header` + `cmp -s`** | Align `write_ustar_entry` / `write_octal` to v2.39.5 field widths; Docker golden | **Recommended** — only Claimed path |
| **A2 Semantic Claimed redefine** | Graduate on tarfile equality | **Refuse** — SoT bar is byte-identical |
| **A3 Tar crate** | Replace hand writer | **Poor** — dialect drift vs pin |
| **A4 Spawn `git archive`** | Product shell-out | **Forbidden** |

**Normative field map (v2.39.5):** mode/uid/gid/devmajor/devminor/chksum → `"%07o"` into 8-byte slots (7 octal digits + NUL); size/mtime → `"%011o"` into 12-byte slots; chksum computed with chksum bytes as spaces; device fields **always** written (never left all-NUL). Reconfirm against https://raw.githubusercontent.com/git/git/v2.39.5/archive-tar.c before coding.

### B — B7 SSH / forge rebind deepen

| Option | Summary | Verdict |
|--------|---------|---------|
| **B1 Same-process dual-bind + drain** (mirror HTTP Claimed) | New listener overlaps old; held SSH/forge sessions complete; new peers hit new bind | **Recommended** for in-process serve |
| **B2 Multi-process FD inheritance** | Cloudflare/Teleport style | Out of scope unless serve becomes multi-process |
| **B3 SO_REUSEPORT alone** | Dual bind without drain evidence | **Refuse as Claimed** — orphan risk documented above |
| **B4 Leave Partial forever** | Honest stop | Allowed product decision; not an implement path |

### C — CLI Partial stubs (ordered deepen, still not invent Claimed)

| Stub | Partial bar (keep) | Claimed bar (future) |
|------|--------------------|----------------------|
| Hub flat leaves depth | Forward/stub + B3 HTTPS remote Claimed | Peer-faithful `gh`-class bodies + matrices STRICT |
| Personality shell | Resolver/Registry/Facade skeleton | Drop-in peer stdout/exit evidence per personality |
| Forge-in-serve depth | Local API scaffold under `arks serve` | Hosting e2e forge module + receipt (≠ B3 client remote alone) |
| Daemon deepen | Flags/path-filter Partial; B5 Partial; B5+ Refuse | Only B5 Claimed reopen above; classic double-fork / facility stay Refuse/Partial |
| Serve SSH process-label | §17(c) Partial label on durable listen | Claimed readiness label **only** when SoT says listen+rebind honesty matches operator surfaces |

### D — TLS residual honesty (B9 already Claimed)

B9 already uses in-process **rustls** PEM terminate. Residual deepen (cert reload without hard-cut, ALPN, client-auth) stays **out of Claimed** until a new SoT row + receipt. Prefer rustls over native-tls for further server work (sources above). Ops reverse-proxy remains valid and does not redefine B9.

## Recommended approach (design-only this goal)

1. Land [[sot:root/arks-remaining-implement-wave]] with ordered backlog, Partial vs Claimed bars, Docker slots, and **“do not implement until SoT+research cited.”**
2. Keep B5 Claimed bar = byte-identical vs pin; Partial structural retained; reopen implement only under that SoT citing this research + [[research:root/20261111084400]].
3. Split B7: HTTP Claimed unchanged; add **B7-SSH** / **B7-forge** Partial→Claimed criteria with `dod-claimed-rebind` scenario matrix (no invent from HTTP receipt).
4. CLI stubs: deepen Spec acceptance only; product waves later Spec-first.
5. Cross-link residual register + claimed-residual gates + verify conventions §7.
6. Public docs honesty MAY sync labels (B5 Partial, B7 HTTP Claimed / SSH·forge Partial) without product code.

## Docker verify slots (no new fourth pillar)

Reuse [[sot:root/arks-claimed-residual-gates]] + [[sot:root/verify-directory-conventions]] §7:

| Concern | Slot | Notes |
|---------|------|-------|
| B5 | `dod-claimed-daemon-archive` | Partial semantic + Claimed `cmp -s`; STRICT fail-closed |
| B7 HTTP | `dod-claimed-rebind` (existing) | Keep Claimed receipt |
| B7 SSH / forge | same script, **separate scenarios** | Default SKIP/Partial until Claimed amend |
| CLI personality / hub depth | symmetry pillars + future `dod-*` only when SoT registers | Not Claimed residual B-ids unless promoted |

## Non-goals

- Product implementation in this research wave.
- Invent Claimed/PASS from SKIP harness or host hex dumps.
- Embed OpenSSH / libssh / russh; product spawn of `git` / `gh` / `ssh`.
- Treat GNU tar chksum folklore or master-branch git as the 2.39.5 oracle.
- Reopen B5+ syslog facility Claimed (terminal Refuse).

## Verification (this research)

1. URLs above reviewed; v2.39.5 `prepare_header` cited from pin tag raw source.
2. Linked from [[sot:root/arks-remaining-implement-wave]] + residual register next-queue.
3. No product Claimed language added beyond existing honesty.
