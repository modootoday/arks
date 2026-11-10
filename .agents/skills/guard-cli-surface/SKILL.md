---
name: guard-cli-surface
description: >
  Use when editing arks CLI nouns, personalities, serve/hub/daemon help, or
  product subprocess boundaries: enforce single-binary personality dispatch,
  serve≠hub, no git/gh spawn, and design≠Claimed honesty for Partial stubs.
---

# Guard CLI surface

## Preconditions

- Read rules `30-no-git-gh-subprocess`, `32-cli-personality`, `33-serve-vs-hub`, `34-ports-normative-vs-demo`, `28-evidence-honesty`, `29-stub-vs-claimed`.
- Owning SoTs: `arks-cli-personality-factory`, `arks-cli-output-exit-contract`, `arks-self-host-server`, `arks-hub-product-intent`, `arks-daemon-product-intent`.
- Related skills: `review-product-subprocess`, `self-host-serve`.

## Checklist

1. Published bin remains **`arks` only**; personalities via env/argv0/shim.
2. Default `arks` = git-first; OverlapFacade input **git→gh**; unified output unless single-peer flags.
3. **`arks serve`** = host; **`arks hub`** = forge client; **`arks daemon`** = git-daemon peer.
4. No `Command::new("git"|"gh")` on product paths.
5. Partial modules fail closed; do not invent Claimed in help or residual.
6. If SoT meaning changes → Spec commit first (rule `27`); leave sibling crate dirt alone if this is a governance lane.

## Done

- [ ] Nouns match owning SoTs
- [ ] Honesty class explicit (design / Partial / Claimed)
- [ ] No Hangul in tracked CLI docs (rule `05`)
