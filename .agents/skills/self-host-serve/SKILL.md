---
name: self-host-serve
description: >
  Use when designing or reviewing arks serve self-host surfaces: serve≠hub,
  normative vs demo ports, fail-closed Partial modules, and serve config vs
  client config. Prefer before editing serve help or hosting docs.
---

# Self-host serve

## Preconditions

- SoT: `.spec/sot/arks-self-host-server.sot.md`
- Rules `33-serve-vs-hub`, `34-ports-normative-vs-demo`, `28`, `29`, `30`
- Ports survey: `.spec/research/20261111055609-git-transport-standard-ports.research.md`
- Related: personality factory, hub/daemon product-intent SoTs

## Checklist

1. **`arks serve`** = product host umbrella (HTTP/SSH/forge modules; `--no-*` opt-outs).
2. **`arks hub …`** = forge **client** — not the primary way to start the host.
3. **`arks daemon`** = `git-daemon` peer — not the GitHub-class host story.
4. Ports: teach **22 / 80 / 443 / 9418** as normative; label **2222 / 8080** demo/alternate.
5. Missing/Partial modules **fail closed** — no silent success; no invent Claimed.
6. `arks serve config` ≠ `arks config` ≠ hub client settings.
7. Governance lane: Spec/rules/skills/docs only unless assigned the crate lane.

## Done

- [ ] Nouns match owning SoTs
- [ ] Port wording honest
- [ ] Honesty class explicit (design / Partial / Claimed)
