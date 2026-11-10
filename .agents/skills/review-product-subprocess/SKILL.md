---
name: review-product-subprocess
description: >
  Use when reviewing Rust product crates for forbidden stock git/gh spawns:
  scan Command::new("git"|"gh"), std::process, and shell wrappers on product
  paths. Prefer before claiming native replacement honesty.
---

# Review product subprocess (no git/gh spawn)

## Preconditions

- Rule `30-no-git-gh-subprocess`.
- Product crates under `crates/**` (especially `arks`, `arks-hub`, `arks-daemon`, `arks-agent`, `arks-server`, `arks-remote`).
- Verify/oracle scripts MAY call stock Git — keep them outside product library paths and label as peer comparison.

## Deny patterns (product paths)

Search and fail closed if found in product runtime code:

```text
Command::new("git")
Command::new("gh")
Command::new("git-daemon")
std::process::Command … "git" / "gh"
shelling out via cmd/sh/pwsh to invoke git or gh for porcelain/hub behavior
```

Allowed (must stay labeled non-product):

- `verify/**`, clean-room matrix scripts, pin/oracle harnesses
- Docs that say stock Git is an **oracle**, not a runtime dependency

## Steps

1. `rg` / search the touched crate(s) for the deny patterns above.
2. Confirm personality / shim paths still execute the **same** `arks` binary (rule `32`).
3. If a spawn is “temporary”: refuse — replace with native stub/Partial fail-closed instead of wrapping git/gh.
4. Report findings; do not invent Claimed native depth (rule `29`).

## Done

- [ ] No product-path git/gh spawn in the reviewed diff
- [ ] Oracle/verify callers clearly non-product
- [ ] Help/docs do not advertise shelling out to git/gh
