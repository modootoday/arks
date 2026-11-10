---
id: adr:root/20261110213500
title: FS3–FS8 full-symmetry slices (patch, filters, replace, LFS closest, GUI handoff)
status: accepted
created: 2026-11-10T21:35:00Z
updated: 2026-11-10T21:35:00Z
tags: [fs3, fs4, fs5, fs6, fs7, fs8, full-symmetry]
rel:
  depends_on:
    - research:root/20261110212414
    - adr:root/20261110212430
    - adr:root/20261110212500
    - sot:root/cli-flag-parity-complete
    - sot:root/arks-remote
    - sot:root/arks-server
---

# FS3–FS8 — Full-symmetry slices

## Decision

| Goal | Claimed slice | Explicit non-claim / impossible |
|------|---------------|----------------------------------|
| **FS3** | Hunk-level `add -p` (y/n/a/d/q/s/e); `add -e`; `add -i` menu (status/update/patch/quit); `stash -p` | Pixel curses chrome identity |
| **FS4** | Filter specs `tree:<depth>`, `blob:limit=<n>` (+k/m/g) in addition to `blob:none`/`tree:0` | Unlisted combine:/sparse: forms until SoT |
| **FS5** | `replace` CLI; `update-server-info` refuses under `info/refs.lock`; local `--bundle-uri` seed | HTTP bundle-uri race harness (NET-006 → FS8 residual) |
| **FS6** | Honor `filter.<name>.clean` when `.gitattributes` sets `filter=`; `scalar` refuse with reason | Full LFS server product; smudge on checkout (next reopen) |
| **FS7** | `gui`/`gitk`/`citool` external-tool handoff refuse; credential helper may be OAuth-capable externally | Embedded Tcl/Tk; in-arks browser OAuth UX |
| **FS8** | Host/env impossibles documented; HOOK-008 closest = `proc-receive.lock` serialize | Full race-freedom Claimed without hostile gate |

## Consequences

Public comparison + residual flip rows to Claimed / Impossible+slice. **FS_final Done** when no *casual* N/A remains — NET-006 HTTP bundle races / OBJ-004 / in-process LFS smudge stay **explicit reopen harnesses**, not convenience N/A. **L9** + live G8 remain non-FS.
