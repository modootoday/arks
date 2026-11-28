# Session overlap (light)

Several LLM chats may touch this repo. Keep coordination **light** until arks adopts a full session board (humanymous has a richer stack — cite, do not copy wholesale).

## Minimum

1. Before non-trivial multi-file edits, check whether another agent already owns the same paths (ask the user / inspect open work).
2. Do not rewrite sibling product SoTs you do not own (`product-intent`, `crate-architecture`, `cli-surface`, Git-compat set) beyond requested cross-links.
3. Prefer disjoint path claims; if conflict is likely, **stop and ask**.
4. In `agy`, work on a fresh task-scoped `agy/<lane>/<slug>` branch per work package or Integrator wave (rule `96`, `.spec/sot/agy-automated-development.sot.md`); legacy Claude sessions use their own `claude/*` branch. Never push foreign branches, force-push (except `--force-with-lease` on your own unmerged branch, rule `92`), or mass-rebase unless the owner explicitly requests it. Sub-agents hold disjoint path claims (or isolated scratch worktrees) and never push (rules `95`, `96`).
5. **Lane split (standing):** follow the Ownership Lanes (`L-core`, `L-cli`, `L-remote`, `L-hub`, `L-serve`, `L-agent`, `L-verify`, `L-gov`) and Hot-File Lock List in `.spec/sot/agy-automated-development.sot.md` §3 — governance / Spec / docs agents own `L-gov` and do not clobber crate lanes (rule `91-commit-hygiene`).
6. When both lanes run: governance commits Spec/rules only; leave sibling crate dirt unstaged.

## Cite

- `.spec/sot/agy-automated-development.sot.md` + rule `96-agy-parallel-governance`
- Full multi-provider board / git-coord patterns (optional future): `C:\workspaces\humanymous\.agents\rules\90-session-overlap.md` and `C:\workspaces\humanymous\.agents\sessions\`.
