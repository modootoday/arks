---
id: sot:root/arks-daemon-git-command-matrix
title: arks daemon ↔ git-daemon command matrix
status: active
created: 2026-11-10T19:24:00Z
updated: 2026-11-25T00:00:00Z
tags: [arks-daemon, git-daemon, matrix, symmetry, clean-room, pin-2.39.5, single-release]
verify: manual
rel:
  depends_on:
    - sot:root/verify-cleanroom-symmetry-matrix
    - sot:root/arks-daemon-product-intent
    - sot:root/cli-surface
    - sot:root/arks-server
    - sot:root/clean-room-git-pin-advance
    - plan:root/20261120000000
    - research:root/20261111035226
  evidence:
    - evidence:root/20261111035226
  governs:
    - verify/matrices/daemon-git-arks-e2e-matrix.csv
---

# arks daemon ↔ git-daemon command matrix

Version **0.3.1**. **Canon** for pin **git-daemon 2.39.5** flag/service/behavior inventory vs **`arks daemon`** on the single published binary. Umbrella: [[sot:root/verify-cleanroom-symmetry-matrix]]. Product shape: [[sot:root/arks-daemon-product-intent]]. Survey: [[research:root/20261111035226]], [[evidence:root/20261111035226]].

**Machine e2e table:** `verify/matrices/daemon-git-arks-e2e-matrix.csv` (living `status_now`). Survey extract: `.spec/evidence/20261111035226-git-daemon-arks-daemon-symmetry/matrix.csv`.

**Pin vs host:** judge = clean-room stock **`git version 2.39.5`**. Host observed **`git version 2.56.0.windows.4`** is skew-only — newer host daemon help MUST NOT redefine pin DoD without a deliberate pin-advance ([[sot:root/clean-room-git-pin-advance]]).

## Rule

1. Every inventoriable pin git-daemon flag/service/behavior from the survey MUST appear in the living CSV with:
   - `kind` — flag | service | behavior | positional | arks-only | pointer
   - `status_now` — Parity | Partial | Missing | Refuse/N/A | arks-only
   - `arks_target` — must-implement | refuse-with-reason | arks-only | pointer
   - `notes` — short honesty
2. **Pointers** (`upload-pack` CLI, `receive-pack` CLI, `git://` client, `serve-http`) are recorded for navigation; they are **not** counted in the daemon flag DoD total (n=**33** inventoriable rows).
3. **arks-only** rows (loopback default bind; the combined `--listen host:port` form is retired by §A5 rule 1) are additive; they do not erase Missing pin flags.
4. Product paths MUST NOT spawn `git`/`gh`. Clean-room may use pin `git daemon` as oracle only.
5. Promoting `status_now` requires tip evidence — **no invent Claimed** from HELP alone.
6. Clean-room gate: compose `dod-daemon-matrix` → `verify/scripts/run-daemon-git-arks-matrix-e2e.sh` (default smoke; `ARKS_DOD_STRICT=1` for Missing=0 DoD).

## Counts (inventoriable + ledger rows; living CSV after A5; was Refuse/N/A 16 before A5, Missing~25 at `c297e34`)

| status_now | Count (living CSV) |
|------------|------:|
| Parity | **37** |
| Partial | **0** |
| Missing | **0** |
| Refuse/N/A | **0** |
| arks-only | 2 |

**A5 (host-tested):** the 16 former Refuse/N/A rows and the retired `--listen host:port` row are `Parity` with CSV note `host-tested` — unit tests in `arks-server` (`daemon::*`), CLI tests `crates/arks/tests/daemon_cli.rs`, and the `ARKS_DAEMON_FLAGS_PARITY=1` A5 block of `verify/scripts/run-daemon-git-arks-matrix-e2e.sh`, which sends identical raw `git://` requests to stock `git daemon` and `arks daemon` and requires identical replies, hook arguments, `--inetd` exit status and argv exit codes. These are host runs (advisory, rule 28); no Docker pin receipt yet, so no Claimed/Verified wording. The paragraphs below describe the pre-A5 state.

**Parity:** inventoriable peer-mapped flags + services (including IW-DAEMON-FLAGS-PARITY deepen — timeouts, listen/port, max-connections, strict-paths, base-path, reuseaddr, enable/disable, dirs, upload-archive/receive-pack, `related:git:// client` **Parity pointer**) per living CSV + [[evidence:root/20261111045108-daemon-flags-parity]] / reconfirm receipts.  
**Partial:** none remaining on living inventoriable daemon rows (flag-depth / platform SO_REUSEADDR notes stay in CSV `notes` where applicable — not a Missing reopen).  
**Refuse/N/A (closed from former Missing~25):** `--detach`, `--inetd`, `--pid-file`, `--user`/`--group`, `--syslog`/`--log-destination` CSV dest row (facility Claimed separately via IW-B5-PLUS collector), `--access-hook`, informative-errors, `--base-path-relaxed`, `--user-path`, `--interpolated-path`, allow/forbid-override, `daemon.*` config, `REMOTE_ADDR` — honest refuse, not invent Claimed.  
Pin judge remains **2.39.5**.

## A5 git-daemon semantics (argv symmetry ladder, [[plan:root/20261120000000]] rung A5)

Owner directive: every `arks daemon` option and behaviour is symmetric with `git daemon`. The
former `Refuse/N/A` rows are `must-implement`. Semantics are derived from
<https://git-scm.com/docs/git-daemon> and from observing stock `git daemon` (oracle only;
no git code copied, no `git` spawned on product paths).

1. **Argv.** Options use git's spelling only: `--opt=<value>` (no separate-value forms),
   option parsing stops at the first non-option or `--`, remaining words are the directory
   whitelist. Unknown options are a usage error (exit 129); `--enable|--disable|--allow-override|
   --forbid-override=<unknown>` is fatal `No such service <name>`; `--log-destination=<bad>` is
   fatal `unknown log destination '<bad>'`. `--listen=<host_or_ipaddr>` is repeatable and is
   resolved like a host name; a combined `host:port` value is not a host and fails to bind,
   exactly as in git (the former arks-only combined form is retired).
2. **Paths.** Requests must start with `/` or `~` and carry no empty, `.` or `..` component
   (`aliased`). `~user[/rest]` needs `--user-path[=<dir>]`. `--interpolated-path` applies only
   when the client sent `host=` and expands `%H` (sanitized, lower-cased host), `%CH`, `%IP`,
   `%P`, `%D`, `%%`. Otherwise `--base-path` is prefixed (`--base-path-relaxed` retries the raw
   path). Non-strict lookup tries the suffixes `/.git`, none, `.git/.git`, `.git`; the validated
   path (unexpanded `~user`, with suffix) is checked against the whitelist (exact under
   `--strict-paths`, otherwise prefix at a `/` boundary).
3. **Services.** Site defaults: upload-pack on, upload-archive and receive-pack off; every
   service is overridable by default and reads `daemon.uploadpack|uploadarch|receivepack`
   from the repository config. Order: site-disabled and not overridable → export marker in the
   git dir → per-repo config → `--access-hook` (service, path, `%H`, `%CH`, `%IP`, `%P`;
   `REMOTE_ADDR`/`REMOTE_PORT`; cwd = repository; first stdout line is the decline message).
4. **Errors.** `--no-informative-errors` (default) answers every refusal with
   `ERR access denied or repository not exported: <dir>`; `--informative-errors` sends
   `no such repository`, `repository not exported`, `service not enabled`, the hook message or
   `service rejected`.
5. **Process.** `--inetd` serves one connection on stdin/stdout (exit 255 on refusal) and is
   incompatible with `--listen`, `--port`, `--detach`, `--user`, `--group`. `--detach` forks
   once, calls `setsid` and points stdio at `/dev/null`; `--pid-file` records the serving
   process (ignored under `--inetd`). `--user=<name>` (names only, primary group unless
   `--group=<name>`; `--group` without `--user` is fatal) drops privileges after the sockets are
   bound. Log destination defaults to `syslog` under `--inetd`/`--detach`, else `stderr`
   (`[pid] message`); syslog writes `<PRI>arks-daemon[pid]: message` datagrams to `/dev/log`
   (facility daemon; info for `--verbose` lines, err for errors). Defaults follow git:
   `--max-connections` 32, `--timeout`/`--init-timeout` 0 (none).
   **Fatal errors** (observed stock `git daemon` 2.39.5, queue unit Q19) follow the log
   destination: `stderr` prints `fatal: <msg>` without the `[pid]` prefix and exits 128;
   `syslog` sends `<msg>` at err and exits 1; `none` is silent and exits 128. A connection
   that closes before its request packet is complete is the fatal
   `the remote end hung up unexpectedly` (for `--inetd`, the process exit above). git serves
   each connection in a process re-executed after privileges are dropped, so every
   connection re-checks `--base-path`: when the serving user cannot see it, the connection
   ends with the fatal `base-path '<path>' does not exist or is not a directory` before any
   log line or reply. The parent's `[<n>] Disconnected[ (with error)]` line names the
   connection by a number (git: the child pid; arks, which serves connections on threads:
   a per-process connection serial).
6. **Default bind (arks-only, unchanged).** git binds every interface when `--listen` is absent;
   `arks daemon` keeps the loopback default `127.0.0.1` (rule 3 and
   [[sot:root/arks-self-host-server]] §Bind). git's behaviour is available explicitly with
   `--listen=0.0.0.0` (and `--listen=::`). Changing the default is an owner decision.
7. Rows move to `Parity` only with host tests (note `host-tested`); Docker pin receipts are
   still required before `Claimed`/`Verified` wording.

## Windows (queue unit Q7)

The pin judge is Linux, so these rules come from <https://git-scm.com/docs/git-daemon> and
from what the Windows platform provides (no `fork`/`setsid`, no POSIX user database, no
syslog). Unix behaviour (§A5) is unchanged. Windows rows carry the CSV note `windows-ci`
when a `cfg(windows)` test on the `windows-2025` CI runner covers them; there is no Windows
pin oracle, so they stay host-tested and never `Claimed`/`Verified`.

1. **`--inetd`** serves one connection on stdin/stdout and exits 255 on refusal, as on Unix.
   stdin is a pipe, not an inherited socket, so the peer address (`REMOTE_ADDR`, peer `%IP`)
   is unknown.
2. **Logging.** git's implied `syslog` destination under `--inetd` has no sink on Windows, so
   the implied destination is `stderr` there. An explicit `--syslog` or
   `--log-destination=syslog` is fatal `syslog is not supported on Windows` (exit 128). A
   Windows Event Log sink would add a Win32 dependency (autonomy level A1) and is not
   implemented.
3. **Process control.** `--detach` is fatal `--detach not supported on this platform`;
   `--user=<name>` (with or without `--group`) is fatal `--user not supported on this platform`;
   `--group` alone stays `--group supplied without --user`. All exit 128 and are checked after
   the `--inetd` incompatibility checks, in the Unix order. The wording follows the Git for
   Windows messages as reported for that platform; it is not observed in the clean-room.
   `--pid-file` works.
4. **Request paths.** The request must still start with `/` or `~`. `\` is a separator as
   well as `/`, so `..\`, `.\` and empty components are `aliased`, and so is any component made
   only of dots and spaces (Win32 trims trailing dots and spaces, so `.. ` could act as `..`).
   Nothing can climb out of `--base-path`.
5. **Base and template.** `--base-path=C:\repos` or `--base-path=\\server\share` is prefixed
   verbatim (`C:\repos/x.git`); `--interpolated-path` expands the same way. `%H` and `%P`
   additionally drop `\` and `:` on Windows, so a client `host=` can not name a drive or a UNC
   root. Without `--base-path`, `/x` is the Windows path `\x` on the current drive; there is no
   `/C:/` translation (git documents none).
6. **Whitelist.** `<directory>...` entries and `--strict-paths` compare with `\` equal to `/`
   and the drive letter case-insensitive (`c:\repos` matches `C:/repos/x.git`); the rest of the
   path compares exactly, with the same `/`-boundary rule as Unix.
7. **`~user`** never resolves on Windows (no passwd database), so `--user-path` requests are
   answered as no such repository.

## Verification

1. CSV inventoriable row count matches survey n=33 (+ optional arks-only/pointer rows clearly tagged).
2. `arks daemon --help` stays honest; Missing flags remain refuse-or-unclaimed until implemented.
3. Default Docker gate is smoke-only while Missing > 0; strict mode fails closed.
4. Public copy never claims full git-daemon parity while Missing > 0.
