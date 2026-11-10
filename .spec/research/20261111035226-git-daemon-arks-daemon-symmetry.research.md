---
id: research:root/20261111035226
title: git-daemon ↔ arks daemon exhaustive symmetry survey
status: active
created: 2026-11-10T18:52:26Z
updated: 2026-11-10T18:52:26Z
tags: [daemon, git-daemon, git://, server, symmetry, inventory, cross-os, no-invent-claimed]
verify: manual
rel:
  depends_on:
    - sot:root/arks-server
    - sot:root/arks-remote
    - sot:root/cli-surface
    - sot:root/clean-room-git-pin-advance
    - research:root/20261111034805
    - research:root/20261111034556
  evidence:
    - evidence:root/20261111035226
---

# git-daemon ↔ arks daemon exhaustive symmetry survey

Tip surveyed: **`fc12914`** (canon `C:\workspaces\gir`; ancestor of `679a202`: yes). **KEEP in-intent** for `arks daemon` (not a trim candidate). **No invent Claimed** — this note is help/docs/implementation inventory only. **No G19** re-stamp.

## Method (Cross-OS)

| Role | Where | How |
|------|-------|-----|
| Pin oracle | Docker `arks-verify-m1:0.1.0` (Linux) | stock **`git version 2.39.5`** (`verify/images.lock.json`) — `git daemon -h` + man `git-daemon.1.gz` via Python extract (no host `sh`) |
| Host skew | Windows host | `git version 2.54.0.windows.1` — `--help` synopsis compared; **not** the parity judge |
| arks surface | Host PE `target/debug/arks.exe` | `daemon --help` + option probes; source `crates/arks/src/cmd/server.rs`, `crates/arks-server/src/daemon.rs` |
| Stale ELF note | `target-cleanroom/release/arks` | Docker exec → `command not implemented: daemon` — **do not** use that ELF as tip surface; host PE matches current sources |

Evidence pack: `.spec/evidence/20261111035226-git-daemon-arks-daemon-symmetry/` (CSV + pin help + probes).

### Classification (this survey)

| Status | Meaning |
|--------|---------|
| **Parity** | Flag/behavior present and semantically aligned with pin git-daemon for the audited axis |
| **Partial** | Present but different shape, subset, or incomplete semantics |
| **Missing** | Pin has it; arks has no accept path / no implementation (CLI: `daemon option not claimed: …` when probed) |
| **Refuse** | Intentional permanent product exclusion — **none** assigned here under KEEP-in-intent (unfinished ≠ refuse) |
| **arks-only** | Shape or companion beyond stock git-daemon flags |

Pointers only (not counted in daemon flag totals): `upload-pack` / `receive-pack` stdio CLIs, `git://` client, `serve-http`.

## Pin git-daemon inventory (2.39.5)

### Synopsis (`git daemon -h` in clean-room)

```
usage: git daemon [--verbose] [--syslog] [--export-all]
           [--timeout=<n>] [--init-timeout=<n>] [--max-connections=<n>]
           [--strict-paths] [--base-path=<path>] [--base-path-relaxed]
           [--user-path | --user-path=<path>]
           [--interpolated-path=<path>]
           [--reuseaddr] [--pid-file=<file>]
           [--(enable|disable|allow-override|forbid-override)=<service>]
           [--access-hook=<path>]
           [--inetd | [--listen=<host_or_ipaddr>] [--port=<n>]
                      [--detach] [--user=<user> [--group=<group>]]
           [--log-destination=(stderr|syslog|none)]
           [<directory>...]
```

### Man SYNOPSIS delta vs `-h`

Pin man (`Git 2.39.5`) **also** documents `--[no-]informative-errors` and expands `--enable=` / `--disable=` / `--allow-override=` / `--forbid-override=` as separate tokens; `-h` compresses the enable family and **omits** informative-errors from the one-line usage (man OPTIONS still normative).

Host 2.54 `-h` synopsis matches pin `-h` for this command (no material host-only daemon flag found in usage line). Host packaged `git-daemon.adoc` aligns with pin man on informative-errors.

### Services (pin)

| Service | Default | Config override |
|---------|---------|-----------------|
| `upload-pack` | enabled | `daemon.uploadpack=false` |
| `upload-archive` | disabled | `daemon.uploadarch=true` |
| `receive-pack` | disabled | `daemon.receivepack=true` |

## arks daemon surface

### Help (`arks daemon --help`)

```
arks daemon — git:// TCP daemon (Wave Sv2 / M25)

Usage:
  arks daemon [--listen ADDR] [--export-all] [--enable=receive-pack] <base-path>

Requires git-daemon-export-ok in each repo unless --export-all.
```

### Accepted CLI

| Flag / arg | Impl wiring |
|------------|-------------|
| `--listen ADDR` | `SocketAddr` parse; default `127.0.0.1:9418` |
| `--export-all` | `DaemonOptions.export_all` |
| `--enable=receive-pack` | `DaemonOptions.allow_receive` |
| `<base-path>` (exactly one) | `DaemonOptions.base_path` |

### Explicit CLI refuse (probe sample)

`--timeout`, `--init-timeout`, `--max-connections`, `--port`, `--strict-paths`, `--base-path=…`, `--base-path-relaxed`, `--user-path`, `--interpolated-path`, `--reuseaddr`, `--pid-file`, `--enable=upload-pack`, `--enable=upload-archive`, `--disable=…`, `--allow-override=…`, `--forbid-override=…`, `--access-hook`, `--inetd`, `--detach`, `--syslog`, `--verbose`, `--log-destination`, `--informative-errors`, `--no-informative-errors`, `--user`, `--group`, `--listen=eq-form` → `error: daemon option not claimed: …`

### Implementation notes (`arks-server` daemon)

- TCP accept loop; one thread per connection; listener nonblocking poll.
- Fixed **30s** read/write socket timeout (not `--timeout` / `--init-timeout`).
- No connection cap (`--max-connections` absent).
- `git-daemon-export-ok` (repo root or `.git/`) unless `export_all`.
- Services: `git-upload-pack` always; `git-receive-pack` if `allow_receive`; no upload-archive.
- Path resolve under `base_path`; `strict_paths` field exists but CLI always passes `false`.
- Initial pkt-line parses `host=` / extras but unused (`dead_code`).
- Export deny → ERR `access denied` (comment cites git `--no-informative-errors`); other failures may send more specific ERR text — **no** informative-errors switch.

## Cross-check matrix (daemon-focused)

Full machine table: [[evidence:root/20261111035226]] `matrix.csv`.

### Counts (git-daemon inventoriable rows, n=33)

| Status | Count |
|--------|------:|
| Parity | **3** |
| Partial | **5** |
| Missing | **25** |
| Refuse | **0** |
| **Sum** | **33** |

Additional **arks-only** (not in pin flag set): **2** (`--listen host:port` combined form; default loopback bind). Related pointers (`upload-pack` / `receive-pack` / `git://` client / `serve-http`) recorded in CSV but **excluded** from the 33.

### Parity (3)

| Item | Note |
|------|------|
| `--export-all` | Same marker bypass |
| service `upload-pack` | Default on |
| `git-daemon-export-ok` | Enforced unless export-all |

### Partial (5)

| Item | Skew |
|------|------|
| `--listen` | Space form `host:port`; not multi-listen; not host-only |
| `--base-path` | Positional base only; no `--base-path=` |
| `<directory>...` | One required base vs optional multi allow-list |
| `--enable=<service>` | Only `--enable=receive-pack` literal |
| service `receive-pack` | Optional enable works; no config/`--disable` matrix |

### Missing (25) — highlight

Listen/ops: `--port`, `--inetd`, `--detach`, `--pid-file`, `--reuseaddr`, `--user`, `--group`.  
Logging: `--verbose`, `--syslog`, `--log-destination`.  
Timeouts/limits: `--timeout`, `--init-timeout`, `--max-connections`.  
Path virtualization: `--strict-paths`, `--base-path-relaxed`, `--user-path`, `--interpolated-path`.  
Service policy: `--disable=*`, `--allow-override=*`, `--forbid-override=*`, `--enable=upload-pack` (redundant), `--enable=upload-archive`, service `upload-archive`, `daemon.*` config overrides.  
Access/errors: `--access-hook`, `--[no-]informative-errors`, `REMOTE_ADDR`.

### Refuse (0)

KEEP-in-intent: unfinished flags are **Missing**, not permanent Refuse.

### arks-only (2+)

| Item | Note |
|------|------|
| Combined `--listen host:port` | git splits `--listen` / `--port` |
| Default `127.0.0.1:9418` | Loopback-first vs typical all-interfaces daemon |

## Behavior checklist (requested axes)

| Axis | Status |
|------|--------|
| listen | Partial |
| port | Missing (via listen only) |
| base-path | Partial |
| export-all | Parity |
| enable=upload-pack | Missing flag (always on = Parity service) |
| enable=receive-pack | Partial |
| enable=upload-archive / archive | Missing |
| access-hook | Missing |
| inetd | Missing |
| syslog / log-destination | Missing |
| timeout / init-timeout | Missing (fixed 30s internal) |
| max-connections | Missing |
| informative-errors | Missing flags; mixed ERR text |
| user-path | Missing |
| interpolated-path | Missing |
| forbid/allow-override | Missing |

## Related server cmds (pointers only)

| Cmd | Role vs daemon |
|-----|----------------|
| `arks upload-pack` / `git-upload-pack` | Stdio duplex; daemon invokes in-process equivalent |
| `arks receive-pack` / `git-receive-pack` | Same for push |
| `arks` / `git` `git://` client | Client lane (R1); peer of daemon |
| `arks serve-http` | HTTP sibling — **not** git-daemon |

## Honesty

- Historical product narrative may stamp daemon wire via G32 scenarios (export-all clone, marker refuse, receive disabled). **This survey does not re-Claim** and does **not** equate G32 wire subset with full git-daemon option parity.
- Clean-room ELF under `target-cleanroom/release/arks` was **stale** for `daemon` on this host; tip judgment used sources + host PE + pin Docker git.

## Korean summary table (counts)

| 구분 | 개수 |
|------|-----:|
| 같다 (Parity) | 3 |
| 일부만 됨 (Partial) | 5 |
| 아직 없음 (Missing) | 25 |
| 일부러 안 함 (Refuse) | 0 |
| arks만 있음 (arks-only) | 2 |
