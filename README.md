# arks for Git

**arks** — **Agent Revision Kinetic System** — is a Git-compatible revision CLI and infra, written in Rust, for environments where **many concurrent sessions** (agents, orchestrators, humans) hit the same repositories.

When tens to hundreds of sessions commit, check out, merge, or move worktrees at once, stock Git’s revision contention becomes the bottleneck. arks keeps the **same `.git` on disk** and a **symmetric CLI**, so you can adopt with `alias git=arks` (or `alias gh=arks`) and keep muscle memory while the engine addresses diverse contention scenarios.

Compatibility with documented Git formats and commands comes first. Clean-room timings exist as evidence ([Performance](docs/performance.md)); this README does not market a blanket “faster than Git” slogan.

> **Runtime vs verification:** End users run native `arks`. Docker clean-room compose is for **maintainer verification** of compatibility and timing claims — not the product install path.
>
> **Honesty:** Design intent and CLI shells are **not** automatic **Claimed**. Surfaces marked **Partial** / **Missing** fail closed or document stubs — this README does not invent PASS.

## Why arks

| Pain | Response |
|------|----------|
| Multi-agent / multi-session orchestration piles onto one revision system | Native Rust engine + product leases and optional high-concurrency shapes aimed at that load |
| Switching tools breaks habits | Git-symmetric CLI; forge client leaves flattened beside porcelain; adopt via alias or PATH shim |
| Parallel proprietary stores break Git tooling | Same filesystem `.git` in Git-compatible mode — stock Git remains a peer |

arks is **not** an agent framework. It is the **revision layer** those environments share. Optional `arks agent …` is an explicit extension on the **same** binary — not the core product story.

More narrative: [docs/](docs/) — especially [Concurrent sessions](docs/concurrent-sessions.md), [Completeness](docs/completion.md), and [Overview](docs/overview.md).

## Install / build

Requirements: a recent Rust toolchain (edition 2024 / `rust-version` **1.85+**).

```bash
git clone https://github.com/modootoday/arks.git
cd arks
cargo build -p arks --release
```

The published binary is **`arks` only** (debug: `target/debug/arks`, release: `target/release/arks`). Put it on your `PATH`, or invoke it with `cargo run -p arks -- …`.

```bash
cargo run -p arks -- --help
```

There is no separate `arks-hub` / `arks-daemon` / `arks-agent` product binary.

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

### Personalities (`git` | `gh` | `arks`)

One binary; three muscle memories (design / Partial — not Claimed drop-in proof):

| How you invoke | Personality | Expectation |
|----------------|-------------|-------------|
| `alias git=arks` / basename `git` / `ARKS_PERSONALITY=git` | **git** | Peer-pure Git argv, stdout, exit codes |
| `alias gh=arks` / basename `gh` / `ARKS_PERSONALITY=gh` | **gh** | Peer-pure forge-client argv, stdout, exit codes |
| Bare `arks` (default) | **arks** | Git-first merged surface; flat forge leaves (`pr`, `issue`, …); unified overlap where designed |

CI that today calls `git` / `gh` should prefer the peer personalities. Details: [CLI personality](docs/cli-personality.md).

### Alias sketch

```bash
alias git=arks
# optional forge peer:
alias gh=arks
```

**Honesty:**

- Treat aliases as the adoption path for claimed commands — not as a promise that every Git/`gh` subcommand and flag is covered today.
- Unimplemented or out-of-scope commands exit **non-zero** and must not mutate the repository.
- Shell aliases do not apply inside many scripts; use a PATH shim named `git` / `gh` when you need non-interactive drop-in behavior.

More detail: [docs/alias-git-arks.md](docs/alias-git-arks.md).

## Self-host (`arks serve`)

Product hosting umbrella — **one** process intended for Git HTTP + SSH + forge API (GitHub-class self-host). **`arks serve` ≠ forge client.** Flat client leaves (`arks pr`, `arks issue`, …) talk *to* a forge base URL; they do not start the host.

```bash
# Narrow Claimed HTTP git wire path (opt out of SSH/forge modules):
arks serve --no-ssh --no-forge /path/to/repos

# Product default shape (HTTP+SSH+forge) — residual B1/B2/B9 Claimed; forge-in-serve still Partial:
# arks serve /path/to/repos
```

| Surface | Role | Tip honesty |
|---------|------|-------------|
| `arks serve` | Host umbrella (`--no-http` / `--no-ssh` / `--no-forge`) | HTTP git wire Claimed where evidenced; SSH **B1/B2 Claimed** ([[evidence:root/20261110235116-claimed-ssh]]); TLS **B9 Claimed** ([[evidence:root/20261110233652-claimed-tls]]); forge-in-serve **Partial** |
| `arks serve config …` | Server config store | **B7 Claimed (HTTP)** rebind ([[evidence:root/20261110233659-claimed-rebind]]); SSH/forge rebind Partial; Windows file-mtime Partial (≠ hub) |
| `arks serve ssh-smoke` | Local SSH banner smoke | **Partial** (smoke ≠ B1/B2 Claimed e2e) |
| `arks daemon` | `git://` peer (`git-daemon` symmetry) | **Partial** deepen; **B5 Partial (terminal)** archive byte-identical; **B5+ Refuse** syslog facility; Windows `--user`/`--group`/`--syslog` Missing refuse |
| Flat `arks pr` / `issue` / `auth` / … | Forge **client** | Leaves **Partial**; **B3** HTTPS forge remote e2e **Claimed** ([[evidence:root/20261110233655-claimed-hub-https]]); `hub smoke` Partial; OUT refuse; never `gh` |
| `arks agent …` | Optional review / CEV extension | **B4 Claimed** fixture e2e ([[evidence:root/20261110231959-claimed-agent-llm]]); smokes Partial; not git-alias DoD |

Ports: prefer **22 / 80 / 443 / 9418**; label **2222 / 8080** as demo/alternate — [Ports](docs/ports.md), [Self-host](docs/self-host.md).

## Symmetry DoD pillars

Public clean-room Definition of Done is **three peer-symmetry pillars** (plus adjacent self-host e2e — not a fourth Claimed pillar):

| Pillar | Stock peer | Product entry |
|--------|------------|---------------|
| **1. CLI** | `git` | `arks <cmd>` / personality `git` |
| **2. Daemon** | `git-daemon` | `arks daemon` |
| **3. Forge client** | `gh` | flat `arks pr` / `issue` / … (transitional `arks hub …`) |

**Completion milestone** for this docs pass means: owning design SoTs + CLI shell are coherent and honesty matches Spec residual rows (**B1–B4 / B7(HTTP) / B9 Claimed**; **B5 Partial**; **B5+ Refuse**) — **not** that every surface is Claimed. See [Completeness](docs/completion.md).

Product paths **never** spawn `git` or `gh`. Stock peers are oracles for clean-room only.

## Common workflows

| You want to… | Try |
|--------------|-----|
| Create a repo | `arks init` |
| Stage / commit | `arks add`, `arks commit -m "…"` |
| Inspect history | `arks status`, `arks log`, `arks show`, `arks diff` |
| Branches | `arks branch`, `arks checkout` / `arks switch`, `arks merge` |
| Remotes | `arks remote`, `arks fetch`, `arks pull`, `arks push`, `arks clone` |
| Sign / verify (SSH) | `arks commit -S` with `gpg.format=ssh`; `arks verify-commit` |
| Self-host (HTTP git) | `arks serve --no-ssh --no-forge <dir>` |
| Forge client (Partial) | `arks pr list`, `arks issue list`, `arks auth status` |

Fuller tables: [docs/workflows.md](docs/workflows.md). For a command list, run `arks --help`.

## Concurrent sessions (honest bar)

arks ships product leases and a **claimed** high-concurrency shape (lease + batch-apply + worktree/ref-shard), plus shared-index N-way work under clean-room receipts. Public copy treats these as **claimed / verified only where receipts exist** — not as “all races fixed.”

Plain-language overview: [docs/concurrent-sessions.md](docs/concurrent-sessions.md).

## Performance (evidence, not slogans)

Clean-room Docker verification compared stock Git **2.39.5** to release `arks` on the journey allow-list. Prefer reading timings as **parallelism / wall-clock evidence for contended workflows**, not as empty speed marketing:

| Bar | Result | Evidence |
|-----|--------|----------|
| ≤0.8× median wall-clock on all journeys | **19/19** | [G19 remeasure `c1b0b24`](.spec/evidence/20261110003836-g19-remeasure-c1b0b24/) · historical [M17](.spec/evidence/20261109090000-m17-exit-g19-measured-19/) |
| Aggressive ≤0.5× | U1 amortize wave + sole remasure ([[evidence:root/20261111003444]]); prior tip **Claimed(partial)**; historical **8/8** at `79fc9bf` | [U-ladder](.spec/evidence/20261111003444-u-ladder-partial-gap-close/) · [tip matrix](.spec/evidence/20261110224814-full-cleanroom-matrix/) · [PF5 8/8 historical](.spec/evidence/20261110142500-g19-remeasure-pf5-8of8/) |

Charts and honesty labels: [docs/performance.md](docs/performance.md). Prefer **ratios** over absolute ms across receipts (seeds change). Host-only timings are advisory only.

## Limits (what is not claimed)

Short and honest — see [docs/limits.md](docs/limits.md) and [Git vs arks](docs/git-vs-arks.md):

- **Not** a GUI embed (`gitk`/`git-gui` handoff only), `scalar` product, agent orchestrator, or “WAN-safe by default” host.
- The Git **object store** remains ordinary filesystem `.git`. Optional index sidecar may sit beside it; arks does not replace `.git` objects in Git-compatible mode.
- Forge **client** leaves and self-host forge-in-serve stay **Partial** except **B3** HTTPS forge remote e2e Claimed — not git-alias DoD.
- In-process **SSH** under `arks serve` is **B1/B2 Claimed** ([[evidence:root/20261110235116-claimed-ssh]]); product TLS **B9 Claimed**; HTTP rebind **B7 Claimed**. Ops SSHD + stdio pack remains valid.
- **Claimed** (with receipts): everyday porcelain inventory where matrices say so; `--depth` shallow; partial-clone filter subset; OpenPGP / X.509 / `tag -s` / `verify-tag`; credential helper / askpass / TTY / first-party OS wallets; smart HTTP server wire (G15 / G29–G32 / G36 Basic); residual **B1/B2/B3/B4/B7(HTTP)/B9** per [arks-claimed-residual-gates](.spec/sot/arks-claimed-residual-gates.sot.md).
- **Advisory / Impossible / blocked:** absolute SHA-1 collision **immunity** Impossible (detect+refuse Claimed); full multi-device sync safety Impossible; live G8 latency SLO needs secrets + receipt (never invent PASS); crates.io timing (**L9**) owner policy; trimmed over-design (`arks host-env` product path, in-process LFS, CELIR as product Claimed path).

## Documentation

Human-facing site (GitHub Pages): [`docs/`](docs/)

| Page | Topic |
|------|--------|
| [Home](docs/index.md) | Motive + product intro |
| [Overview](docs/overview.md) | What arks is (and is not) |
| [Completeness](docs/completion.md) | Completion milestone — what “done” means (honest Partial) |
| [Getting started](docs/getting-started.md) | Native build and first commands |
| [CLI personality](docs/cli-personality.md) | `git` / `gh` / `arks` dispatch |
| [Alias `git` → `arks`](docs/alias-git-arks.md) | Drop-in adoption path |
| [Workflows](docs/workflows.md) | Everyday command map |
| [Self-host](docs/self-host.md) | `arks serve` GitHub-class host |
| [Ports](docs/ports.md) | Normative **22/80/443/9418** vs demo **2222/8080** |
| [Forge client](docs/arks-hub.md) | Flat `arks pr` / … (≠ host) |
| [Daemon](docs/daemon.md) | `arks daemon` ↔ `git-daemon` |
| [Agent](docs/agent.md) | Optional `arks agent …` Partial |
| [Concurrent sessions](docs/concurrent-sessions.md) | Contention motive and concurrency honesty |
| [Git vs arks](docs/git-vs-arks.md) | Honest feature comparison |
| [Performance](docs/performance.md) | Clean-room git vs arks timings |
| [Limits](docs/limits.md) | What is not claimed |
| [Contribute](docs/contribute.md) | Bugs, docs, maintainer pointers |

Design specs, verification matrices, and agent routing live under [`.spec/`](.spec/), [`verify/`](verify/), and [`AGENTS.md`](AGENTS.md) — they are for maintainers, not the default product read.

## Contributing

Bug reports and pull requests are welcome on [modootoday/arks](https://github.com/modootoday/arks). Keep product docs in English. See [docs/contribute.md](docs/contribute.md). Maintainers: start from [`AGENTS.md`](AGENTS.md).

## License and security

- License: **Apache-2.0 OR MIT** (`LICENSE`, `LICENSE-APACHE`, `LICENSE-MIT`)
- Vulnerability reports: [`SECURITY.md`](SECURITY.md) (private GitHub Security Advisories)
