---
id: evidence:root/20261110213530
title: FS3–FS8 host smoke + unit evidence
status: active
created: 2026-11-10T21:35:30Z
updated: 2026-11-10T21:35:30Z
tags: [fs3, fs4, fs5, fs6, fs7, fs8, full-symmetry]
rel:
  depends_on:
    - adr:root/20261110213500
    - research:root/20261110212414
  evidence: []
files: []
---

# FS3–FS8 evidence

| Slice | How | Result |
|-------|-----|--------|
| FS3 `add -p` hunk | `ARKS_ADD_PATCH_ANSWERS=yn` on multi-hunk file | **PASS** (host) |
| FS3 `add -i` | `ARKS_ADD_INTERACTIVE_ANSWERS=status/quit` | **PASS** |
| FS3 `stash -p` | answers `y` then stash list | **PASS** |
| FS2 regress | `arks-server` unit advertise proc-receive | **PASS** |
| FS4 filter parse | `validate_claimed_filter` accepts `blob:limit=1k` / `tree:1` | **Claimed** (code + remote tests green) |
| FS5 replace / bundle-uri / info lock | CLI + refuse paths wired | **Claimed** (host compile + SoT) |
| FS6 clean filter | attribute + config clean spawn on add | **Claimed** (wire) |
| FS7 GUI handoff | `arks gui` / `gitk` / `citool` / `scalar` refuse messages | **Claimed** |

Non-claims: HTTP bundle-uri races, HOOK-008 hostile, smudge-on-checkout, full LFS server, embedded GUI.
