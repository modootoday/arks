# arks for Git

**arks** — **Agent Revision Kinetic System** — is a Git-compatible revision CLI and infra, written in Rust, for environments where **many concurrent sessions** (agents, orchestrators, humans) hit the same repositories.

When tens to hundreds of sessions commit, check out, merge, or move worktrees at once, stock Git’s revision contention becomes the bottleneck. arks keeps the **same `.git` on disk** and a **fully symmetric CLI**, so you can adopt with `alias git=arks` and keep muscle memory while the engine addresses diverse contention scenarios.

Compatibility with documented Git formats and commands comes first. Clean-room timings exist as evidence ([Performance](docs/performance.md)); this README does not market a blanket “faster than Git” slogan.

> **Runtime vs verification:** End users run native `arks`. Docker clean-room compose is for **maintainer verification** of compatibility and timing claims — not the product install path.

## Why arks

| Pain | Response |
|------|----------|
| Multi-agent / multi-session orchestration piles onto one revision system | Native Rust engine + product leases and optional high-concurrency shapes aimed at that load |
| Switching tools breaks habits | Git-symmetric CLI; adopt via alias or PATH shim |
| Parallel proprietary stores break Git tooling | Same filesystem `.git` in Git-compatible mode — stock Git remains a peer |

arks is **not** an agent framework. It is the **revision layer** those environments share.

More narrative: [docs/](docs/) — especially [Concurrent sessions](docs/concurrent-sessions.md) and [Overview](docs/overview.md).

## Install / build

Requirements: a recent Rust toolchain (edition 2024 / `rust-version` **1.85+**).

```bash
git clone https://github.com/modootoday/arks.git
cd arks
cargo build -p arks --release
```

The binary is `arks` (debug: `target/debug/arks`, release: `target/release/arks`). Put it on your `PATH`, or invoke it with `cargo run -p arks -- …`.

```bash
cargo run -p arks -- --help
```

## Quick start

```bash
arks init my-repo
cd my-repo
echo "hello" > README.md
arks add README.md
arks commit -m "Initial commit"
arks status
arks log
```

Remote day-to-day (smart HTTP):

```bash
arks clone https://example.com/repo.git
arks fetch
arks pull
arks push
```

Same repository as Git: you can inspect the tree with stock `git` anytime — arks does not invent a separate object store for Git-compatible mode.

## Alias `git` → `arks`

The published binary stays **`arks`**. Drop-in usage is a shell alias or a PATH shim, not a renamed binary:

```bash
alias git=arks
```

Then familiar commands (`git status`, `git commit -m …`) invoke arks with the same arguments.

**Honesty:**

- Treat the alias as the adoption path for claimed commands — not as a promise that every Git subcommand and flag is covered today.
- Commands or flags that are unimplemented or explicitly out of scope exit **non-zero** and must not mutate the repository.
- Shell aliases do not apply inside many scripts; use a PATH shim named `git` when you need non-interactive drop-in behavior.

More detail: [docs/alias-git-arks.md](docs/alias-git-arks.md).

## Common workflows

| You want to… | Try |
|--------------|-----|
| Create a repo | `arks init` |
| Stage / commit | `arks add`, `arks commit -m "…"` |
| Inspect history | `arks status`, `arks log`, `arks show`, `arks diff` |
| Branches | `arks branch`, `arks checkout` / `arks switch`, `arks merge` |
| Remotes | `arks remote`, `arks fetch`, `arks pull`, `arks push`, `arks clone` |
| Sign / verify (SSH) | `arks commit -S` with `gpg.format=ssh`; `arks verify-commit` |

Fuller tables: [docs/workflows.md](docs/workflows.md). For a command list, run `arks --help`.

## Concurrent sessions (honest bar)

arks ships product leases and a **claimed** high-concurrency shape (lease + batch-apply + worktree/ref-shard), plus shared-index N-way work under clean-room receipts. Public copy treats these as **claimed / verified only where receipts exist** — not as “all races fixed.”

Plain-language overview: [docs/concurrent-sessions.md](docs/concurrent-sessions.md).

## Performance (evidence, not slogans)

Clean-room Docker verification compared stock Git **2.39.5** to release `arks` on the journey allow-list. Prefer reading timings as **parallelism / wall-clock evidence for contended workflows**, not as empty speed marketing:

| Bar | Result | Evidence |
|-----|--------|----------|
| ≤0.8× median wall-clock on all journeys | **19/19** | [G19 remeasure `c1b0b24`](.spec/evidence/20261110003836-g19-remeasure-c1b0b24/) · historical [M17](.spec/evidence/20261109090000-m17-exit-g19-measured-19/) |
| Aggressive ≤0.5× | Tip `54c9dc2` **Claimed(partial)** (17 + ALIAS/REVERT `aggressive_na`); historical **8/8** at `79fc9bf` | [tip matrix](.spec/evidence/20261110224814-full-cleanroom-matrix/) · [PF5 8/8 historical](.spec/evidence/20261110142500-g19-remeasure-pf5-8of8/) · prior [875359e](.spec/evidence/20261110131600-g19-remeasure-875359e-pf5-8of8/) |

Charts and honesty labels: [docs/performance.md](docs/performance.md). Prefer **ratios** over absolute ms across receipts (seeds change). Host-only timings are advisory only.

## Limits (what is not claimed)

Short and honest — see [docs/limits.md](docs/limits.md) and [Git vs arks](docs/git-vs-arks.md):

- **Not** a GUI embed (`gitk`/`git-gui` handoff only), `scalar` product, forge host API tool (Issues/PRs/Checks — use `gh`), or agent orchestrator.
- The Git **object store** remains ordinary filesystem `.git`. An optional index sidecar may sit beside it for product queries; arks does not replace `.git` objects with a different store in Git-compatible mode.
- **Claimed** (with receipts): `--depth` shallow; partial-clone `blob:none` / `tree:0` / related filter subset; OpenPGP / X.509 / `tag -s` / `verify-tag`; in-process Git LFS fetch/push + smudge; credential helper / askpass / TTY / first-party OS wallets.
- **Advisory / Impossible (documented non-goals):** `arks host-env` cloud-sync / OneDrive CfAPI is **detect+advisory only** (full multi-device sync safety Impossible); absolute SHA-1 collision **immunity** Impossible (detect+refuse Claimed); forge OAuth browser UX Impossible; live G8 latency SLO needs secrets + receipt (never invent PASS); crates.io timing (**L9**) remains owner policy.

## Documentation

Human-facing site (GitHub Pages): [`docs/`](docs/)

| Page | Topic |
|------|--------|
| [Home](docs/index.md) | Motive + product intro |
| [Overview](docs/overview.md) | What arks is (and is not) |
| [Getting started](docs/getting-started.md) | Native build and first commands |
| [Alias `git` → `arks`](docs/alias-git-arks.md) | Drop-in adoption path |
| [Workflows](docs/workflows.md) | Everyday command map |
| [Concurrent sessions](docs/concurrent-sessions.md) | Contention motive and concurrency honesty |
| [Git vs arks](docs/git-vs-arks.md) | Honest feature comparison (Claimed / N/A / reopen) |
| [Completeness](docs/completion.md) | FS_final YES vs documented non-goals (PARTIAL overall) |
| [Performance](docs/performance.md) | Clean-room git vs arks timings |
| [Limits](docs/limits.md) | What is not claimed |
| [Contribute](docs/contribute.md) | Bugs, docs, maintainer pointers |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Root pointer to contribute guidance |

Design specs, verification matrices, and agent routing live under [`.spec/`](.spec/), [`verify/`](verify/), and [`AGENTS.md`](AGENTS.md) — they are for maintainers, not the default product read.

## Contributing

Bug reports and pull requests are welcome on [modootoday/arks](https://github.com/modootoday/arks). Keep product docs in English. See [docs/contribute.md](docs/contribute.md). Maintainers: start from [`AGENTS.md`](AGENTS.md).

## License and security

- License: **Apache-2.0 OR MIT** (`LICENSE`, `LICENSE-APACHE`, `LICENSE-MIT`)
- Vulnerability reports: [`SECURITY.md`](SECURITY.md) (private GitHub Security Advisories)
