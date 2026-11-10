# `arks serve` ≠ `arks hub`

Hosting and forge-client are different nouns. Do not conflate them in help, SoT, or commits.

## Roles

| Noun | Role | Owning SoT |
|------|------|------------|
| **`arks serve`** | Product **host** umbrella (Git HTTP/SSH + forge API modules; `--no-*` opt-outs; nested `serve config`) | `.spec/sot/arks-self-host-server.sot.md` |
| **`arks hub …`** | **`gh`-like forge client** against a forge base URL (transitional nested group; flat leaves preferred) | `.spec/sot/arks-hub-product-intent.sot.md` |
| **`arks daemon`** | **`git-daemon` symmetry** — not the forge host | `.spec/sot/arks-daemon-product-intent.sot.md` |

## Must

- Teach **`arks serve`** as the self-host story; `arks hub serve` / `hub listen` are **thin transitional aliases** only.
- Keep hub work focused on **client** leaves/API; keep serve work focused on **listeners / forge-in-serve host module**.
- Fail-closed when an enabled serve module is Missing/Partial — no silent “success” (rule `28`).
- Distinguish `arks serve config` (server) from `arks config` / hub client settings.

## Must not

- Document `arks hub` as the primary way to start the host.
- Treat transitional `arks hub serve` Partial scaffold as forge-in-serve Claimed.
- Merge daemon flag-parity matrices into serve hosting Claimed.
- Rewrite sibling-owned serve/hub **crate** bodies from a governance-only session — Spec/rules/skills only unless the user assigns that lane.

## Cite

- `.spec/sot/arks-cli-personality-factory.sot.md`
- skill `guard-cli-surface`
