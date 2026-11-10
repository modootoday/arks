---
id: sot:root/arks-daemon-git-command-matrix
title: arks daemon ↔ git-daemon command matrix
status: active
created: 2026-11-10T19:24:00Z
updated: 2026-11-10T19:30:00Z
tags: [arks-daemon, git-daemon, matrix, symmetry, clean-room, pin-2.39.5, single-release]
verify: manual
rel:
  depends_on:
    - sot:root/verify-cleanroom-symmetry-matrix
    - sot:root/arks-daemon-product-intent
    - sot:root/cli-surface
    - sot:root/arks-server
    - sot:root/clean-room-git-pin-advance
    - research:root/20261111035226
  evidence:
    - evidence:root/20261111035226
  governs:
    - verify/matrices/daemon-git-arks-e2e-matrix.csv
---

# arks daemon ↔ git-daemon command matrix

Version **0.1.1**. **Canon** for pin **git-daemon 2.39.5** flag/service/behavior inventory vs **`arks daemon`** on the single published binary. Umbrella: [[sot:root/verify-cleanroom-symmetry-matrix]]. Product shape: [[sot:root/arks-daemon-product-intent]]. Survey: [[research:root/20261111035226]], [[evidence:root/20261111035226]].

**Machine e2e table:** `verify/matrices/daemon-git-arks-e2e-matrix.csv` (living `status_now`). Survey extract: `.spec/evidence/20261111035226-git-daemon-arks-daemon-symmetry/matrix.csv`.

**Pin vs host:** judge = clean-room stock **`git version 2.39.5`**. Host observed **`git version 2.56.0.windows.4`** is skew-only — newer host daemon help MUST NOT redefine pin DoD without a deliberate pin-advance ([[sot:root/clean-room-git-pin-advance]]).

## Rule

1. Every inventoriable pin git-daemon flag/service/behavior from the survey MUST appear in the living CSV with:
   - `kind` — flag | service | behavior | positional | arks-only | pointer
   - `status_now` — Parity | Partial | Missing | Refuse/N/A | arks-only
   - `arks_target` — must-implement | refuse-with-reason | arks-only | pointer
   - `notes` — short honesty
2. **Pointers** (`upload-pack` CLI, `receive-pack` CLI, `git://` client, `serve-http`) are recorded for navigation; they are **not** counted in the daemon flag DoD total (n=**33** inventoriable rows).
3. **arks-only** rows (combined `--listen host:port`, loopback default bind) are additive; they do not erase Missing pin flags.
4. Product paths MUST NOT spawn `git`/`gh`. Clean-room may use pin `git daemon` as oracle only.
5. Promoting `status_now` requires tip evidence — **no invent Claimed** from HELP alone.
6. Clean-room gate: compose `dod-daemon-matrix` → `verify/scripts/run-daemon-git-arks-matrix-e2e.sh` (default smoke; `ARKS_DOD_STRICT=1` for Missing=0 DoD).

## Counts (inventoriable + ledger rows; living CSV tip; was Missing~25 at `c297e34`)

| status_now | Count (living CSV) |
|------------|------:|
| Parity | 5 |
| Partial | 15 |
| Missing | **0** |
| Refuse/N/A | 16 |
| arks-only | 3 |

**Parity:** `--export-all`, service `upload-pack`, `git-daemon-export-ok`, (+ related ledger Parity rows).  
**Partial (deepened):** separate `--init-timeout` vs `--timeout`; max-conn accept-then-drop; `--strict-paths` disables foo↔foo.git alias; multi `--base-path`/positionals allow-list; `--listen`/`--port`; `--reuseaddr` flag (platform SO_REUSEADDR Partial); enable/disable services; upload-archive accept+ERR.  
**Refuse/N/A (closed from former Missing~25):** `--detach`, `--inetd`, `--pid-file`, `--user`/`--group`, `--syslog`/`--log-destination`, `--access-hook`, informative-errors, `--base-path-relaxed`, `--user-path`, `--interpolated-path`, allow/forbid-override, `daemon.*` config, `REMOTE_ADDR` — honest refuse, not Claimed.  
Deepen Partial→Parity still open (esp. reuseaddr setsockopt on Windows, upload-archive protocol); see CSV notes. Pin judge remains **2.39.5**.

## Verification

1. CSV inventoriable row count matches survey n=33 (+ optional arks-only/pointer rows clearly tagged).
2. `arks daemon --help` stays honest; Missing flags remain refuse-or-unclaimed until implemented.
3. Default Docker gate is smoke-only while Missing > 0; strict mode fails closed.
4. Public copy never claims full git-daemon parity while Missing > 0.
