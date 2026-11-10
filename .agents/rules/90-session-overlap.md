# Session overlap (light)

Several LLM chats may touch this repo. Keep coordination **light** until arks adopts a full session board (humanymous has a richer stack — cite, do not copy wholesale).

## Minimum

1. Before non-trivial multi-file edits, check whether another agent already owns the same paths (ask the user / inspect open work).
2. Do not rewrite sibling product SoTs you do not own (`product-intent`, `crate-architecture`, `cli-surface`, Git-compat set) beyond requested cross-links.
3. Prefer disjoint path claims; if conflict is likely, **stop and ask**.
4. Do not create branches, force-push, or mass-rebase unless the user explicitly requests it.
5. **Lane split (standing):** governance / Spec / docs agents prefer `.agents/`, `AGENTS.md`, `.spec` governance SoT, hooks/skills; CLI stub implementation agents own `crates/**` — do not clobber each other (rule `91-commit-hygiene`).
6. When both lanes run: governance commits Spec/rules only; leave sibling crate dirt unstaged.

## Cite

Full multi-provider board / git-coord patterns (optional future): `C:\workspaces\humanymous\.agents\rules\90-session-overlap.md` and `C:\workspaces\humanymous\.agents\sessions\`.
