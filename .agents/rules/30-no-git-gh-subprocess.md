# No `git` / `gh` subprocess on product paths

Published product paths MUST NOT spawn stock `git` or `gh` to implement behavior. arks is a native replacement CLI, not a wrapper.

## Must

- Implement Git-compat and forge-client behavior in Rust crates (`arks`, `arks-hub`, `arks-daemon`, `arks-agent`, …).
- Use personality / argv0 / shim for drop-in UX (`git`→personality `git`, `gh`→personality `gh`) — still the **same** `arks` binary (rule `32-cli-personality`).
- Keep verify harnesses that intentionally call stock Git **outside** product library paths (clean-room / matrix scripts), clearly labeled as peer comparison — not product runtime.

## Must not

- `Command::new("git")` / `Command::new("gh")` (or shell equivalents) from product crates to satisfy porcelain, hub, daemon, serve, or agent features.
- Document “shells out to git/gh” as the shipping architecture.
- Publish a second bin named `git` or `gh` (product naming SoT).

## Cite

- `.spec/README.md` (Absolute: never spawn `git` or `gh`)
- `.spec/sot/product-intent.sot.md`
- `.spec/sot/arks-cli-personality-factory.sot.md`
- skill `guard-cli-surface`
