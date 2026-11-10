---
id: evidence:root/20261110213330
title: FS3–FS8 Full Symmetry slices host/unit evidence
status: active
created: 2026-11-10T21:33:30Z
updated: 2026-11-10T21:33:30Z
tags: [fs3, fs4, fs5, fs6, fs7, fs8, full-symmetry]
rel:
  depends_on:
    - adr:root/20261110213300
    - research:root/20261110212414
  evidence: []
files: []
---

# FS3–FS8 evidence

| Slice | Exercise | Result |
|-------|----------|--------|
| FS3 hunk `add -p` | Unit `hunk_split_and_apply_partial` + host `ARKS_ADD_PATCH_ANSWERS=yn` | **PASS** |
| FS3 `stash -p` | Host `ARKS_ADD_PATCH_ANSWERS=y` | **PASS** |
| FS4 `blob:limit` | `parse_filter("blob:limit=1k")` unit | **PASS** |
| FS5 `replace` | Module + CLI dispatch | **Claimed** (host) |
| FS5 `--bundle-uri` local | Clone path parses + unbundle seed | **Claimed** |
| FS5 update-server-info lock | Refuse under `info/refs.lock` | **Claimed** |
| FS6 `arks lfs` | External handoff / honest missing-binary | **Claimed** closest |
| FS7 GUI/scalar refuse | main dispatch messages | **Claimed** handoff |
| FS8 proc-receive.lock | Server refuse when lock held | **Claimed** closest |

Clean-room Docker twins for FS3–FS5 may expand later; host/unit evidence carries Claimed under Specfold honesty for these slices.
