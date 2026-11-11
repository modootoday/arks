---
id: research:root/20261111093543
title: Partial residual wave — web research (forge cohost, hub flat, personality drop-in, daemon flags)
status: active
created: 2026-11-11T00:35:43Z
updated: 2026-11-11T00:35:43Z
tags: [research, partial, residual, forge-in-serve, hub-flat, personality, daemon, syslog, detach, hosting-e2e, no-invent-claimed, docker]
verify: manual
rel:
  depends_on:
    - research:root/20261111090827
    - research:root/20261111072342
    - research:root/20261111084400
    - sot:root/arks-remaining-implement-wave
    - sot:root/arks-partial-residual-wave
    - sot:root/arks-self-host-server
    - sot:root/arks-cli-personality-factory
    - sot:root/arks-cli-output-exit-contract
    - sot:root/arks-hub-cli-surface
    - sot:root/arks-daemon-product-intent
    - sot:root/arks-claimed-residual-gates
    - sot:root/verify-directory-conventions
    - sot:root/product-verification-clean-room
  related:
    - verify/scripts/run-self-host-e2e.sh
    - crates/arks/src/cmd/server.rs
    - crates/arks/src/factory/exit_code.rs
    - crates/arks-hub/src/lib.rs
---

# Partial residual wave — web research

English-only. Tip surveyed: **`ff39b69`**. **Design ≠ Claimed.** Do **not** implement product Claimed from this note alone — cite this research + [[sot:root/arks-partial-residual-wave]] before crate changes.

Prior wave closed IW-B5 / IW-B7-SSH / IW-B7-FORGE / IW-SSH-LABEL as Claimed ([[research:root/20261111090827]], [[sot:root/arks-remaining-implement-wave]]). Four **Partial leftovers** remain:

| ID | Tip honesty | Claimed still Missing / Refuse |
|----|-------------|-------------------------------|
| **IW-FORGE-SERVE** | Default `arks serve` cohosts HTTP+SSH+forge | Hosting e2e PR/issue host journeys |
| **IW-HUB-FLAT** | Flat leaves + hub forwards Partial stubs; **B3** Claimed | Peer-faithful leaf bodies + hub matrix STRICT |
| **IW-PERSONALITY** | Resolver/Registry/OverlapFacade + ExitCodePolicy OR-of-failure | Peer-native stdout/exit drop-in for `git`/`gh` |
| **IW-DAEMON-FLAGS** | path-filter / `--syslog` openlog / `--user`/`--group` / detach Partial | Classic double-fork Claimed; syslog **facility/priority** Claimed (**B5+ Refuse**) |

## Sources (web / pin peers)

| Topic | URL | Fact used |
|-------|-----|-----------|
| Forgejo single-process cohost | https://stdout.adegoodyer.com/posts/how-forgejo-serves-a-team | One binary serves web UI + REST API + git HTTPS + built-in SSH; HTTP≠SSH ports; forge API is **host** surface, not client remote smoke |
| Forgejo admin SSH/HTTP | https://forgejo.org/docs/latest/admin/config-cheat-sheet/ | `START_SSH_SERVER`, `DISABLE_SSH`, separate `HTTP_*` / `SSH_*` listen; cohost is deliberate product shape |
| Forgejo architecture | https://forgejo.org/docs/latest/contributor/architecture/ | Git transport vs forge DB environment (issues/PRs) are distinct layers in one process |
| GitHub-compat hosting DoD | https://github.com/tamnd/githome | Claimed forge host = unmodified `gh`/`git` against server; REST/GraphQL shapes; **client remote Claimed ≠ server hosting Claimed** |
| Differential / conform | https://github.com/tamnd/github-compat | Host journeys assert server fields vs github.com; client smoke alone insufficient |
| Stateful forge e2e pattern | https://github.com/qinqon/oompa/pull/220 | Hosting e2e = seed issue → PR lifecycle against **server** mock/host; black-box outcomes |
| Forge conformance tiers | https://docs.kitedeck.ai/guides/forge-conformance/ | In-memory double ≠ real host; Claimed needs host-tier evidence |
| Self-hosted forge integration | https://github.com/prata0x/gfo/blob/main/docs/integration-testing.md | Docker Compose spins forge hosts; PR/issue/label journeys are the matrix |
| `gh` exit codes | https://cli.github.com/manual/gh_help_exit-codes | Drop-in: 0/1/2/4 classes; command-specific extras possible |
| `gh` scriptability | https://github.com/cli/cli/blob/trunk/docs/command-development.md | Preserve flags, exit, stdout/stderr routing, JSON fields for drop-in |
| `gh` alias exit propagation | https://github.com/cli/cli/blob/v1.14.0/cmd/gh/main.go ; https://cli.github.com/manual/gh_alias_set | Alias/shim drop-in must propagate peer exit + streams — not invent unified OR under `gh` personality |
| `git-daemon` options | https://git-scm.com/docs/git-daemon | `--syslog` ≡ `--log-destination=syslog`; `--detach` implies syslog; `--user`/`--group`; inetd incompatibilities |
| `git-daemon` openlog | https://github.com/git/git/blob/master/daemon.c ; https://github.com/git/git/commit/0c591cacba00a36af797fa3b6c4a4f12d342be86 | `openlog("git-daemon", LOG_PID, LOG_DAEMON)` when syslog destination selected |
| syslog facility vs level | https://man7.org/linux/man-pages/man3/syslog.3.html | Facility (`LOG_DAEMON`) ≠ level (`LOG_INFO`…); priority = facility\|level; Claimed facility/priority needs collector receipt |
| git `daemonize()` | https://public-inbox.org/git/1391843332-20583-1-git-send-email-pclouds@gmail.com/ | Stock detach = **single fork** + `setsid` + close stdio — **not** classic SysV double-fork |
| Dual-bind / drain (prior) | https://blog.cloudflare.com/ecdysis-rust-graceful-restarts/ ; https://github.blog/news-insights/glb-part-2-haproxy-zero-downtime-zero-delay-reloads-with-multibinder/ | Already gated B7 Claimed; hosting e2e is a **different** Claimed class |

Clean-room pin remains Git **2.39.5** ([[sot:root/product-verification-clean-room]]).

## Failure modes — why inventing Claimed fails here

| Mode | What goes wrong | Prevention |
|------|-----------------|------------|
| **B3 over-read** | Hub HTTPS **client** remote Claimed treated as forge-in-serve / flat-leaf Claimed | Split rows: B3 ≠ IW-FORGE-SERVE ≠ IW-HUB-FLAT |
| **Cohost smoke = hosting e2e** | Help text / health OK while PR/issue host journeys absent | Claimed bar = PR/issue host journeys under `dod-self-host` (or §7 `dod-claimed-*`) with receipt |
| **ExitCodePolicy unit = drop-in** | OR-of-failure unit green marketed as `git`/`gh` personality Claimed | Drop-in Claimed needs peer stdout/exit evidence per personality SoT |
| **openlog = facility Claimed** | `LOG_DAEMON` arming without collector facility/priority compare | B5+ **Refuse** until deliberate collector wave |
| **Detach = double-fork Claimed** | Re-exec / single-fork Partial sold as classic double-fork | Peer is single-fork+setsid; classic double-fork stays **Refuse**; Partial detach stays labeled Partial |
| **Hub stub green = leaf Claimed** | Matrix SKIP / stub exit 2 treated as peer-faithful bodies | STRICT hub matrices + peer-faithful stdout only |

## Options by residual class

### A — IW-FORGE-SERVE (hosting depth)

| Option | Summary | Verdict |
|--------|---------|---------|
| **A1 Register hosting e2e Claimed bar + deepen scaffold Partial** | Lock PR/issue host journeys; enhance `run-self-host-e2e.sh` cohost probe; STRICT fail-closed | **Recommended** this wave |
| **A2 Invent Claimed from cohost + B3** | Graduate without host journeys | **Refuse** |
| **A3 Embed OpenSSH / spawn `gh` against local forge** | Ops/client shortcuts | **Forbidden** for product Claimed |

**Claimed bar (future):** Docker `dod-self-host` (or registered `dod-claimed-forge-host`) proves arks-only `arks serve` cohost + at least one PR create/list and one issue create/list against the **local forge module** with scrubbed receipt. **B3 client remote Claimed alone ≠ this row.**

### B — IW-HUB-FLAT (flat-leaf depth)

| Option | Summary | Verdict |
|--------|---------|---------|
| **B1 Keep Partial stubs; Spec honesty vs B3** | Flat leaves remain Partial; hub pillar matrices STRICT for Claimed | **Recommended** |
| **B2 Graduate leaves from B3 receipt** | | **Refuse** |
| **B3 Full gh matrix Claimed this wave** | Peer-faithful bodies for all v1 leaves | Out of scope unless separate Claimed wave + receipts |

**Hub flat ≠ serve:** Flat leaves are **client** argv (`arks pr …`); `arks serve` forge module is **host**. Transitional `arks hub serve` ≠ IW-FORGE-SERVE Claimed.

### C — IW-PERSONALITY (drop-in)

| Option | Summary | Verdict |
|--------|---------|---------|
| **C1 Keep skeleton + OR ExitCodePolicy Partial** | Unit-tested OR-of-failure; drop-in Claimed deferred | **Recommended** |
| **C2 Claim drop-in from ExitCodePolicy units** | | **Refuse** |
| **C3 Spawn stock `git`/`gh` for drop-in** | | **Forbidden** |

**Claimed bar (future):** Under `ARKS_PERSONALITY=git` / `gh` (or argv0 shim), selected porcelain/leaves match peer stdout shape + exit classes (cite `gh` exit-codes + git peer) with CLI symmetry matrix receipt. Unified `arks` OR policy stays a **third** personality — not CI drop-in.

### D — IW-DAEMON-FLAGS / B5+

| Option | Summary | Verdict |
|--------|---------|---------|
| **D1 Close deepen; keep Partial flags; Refuse Claimed double-fork + facility** | Align help/SoT with pin facts | **Recommended** |
| **D2 Claimed syslog facility via openlog alone** | | **Refuse** (B5+ terminal) |
| **D3 Claimed classic double-fork** | | **Refuse** — peer is single-fork+setsid; classic double-fork not the pin |

**Normative peer facts:** stock `--detach` → `daemonize()` = fork once + `setsid` + sanitize stdio; syslog → `openlog(..., LOG_DAEMON)`. arks Partial `--detach` (re-exec) and `--syslog` (openlog) stay **Partial**. Archive Claimed is **IW-B5 only**.

## Recommended approach (this wave)

1. Land [[sot:root/arks-partial-residual-wave]] with DoD bars, order, non-goals, verify gates.
2. Phase B implement **only** SoT-mandated Partial deepen / honesty — prefer Partial/Refuse over invent Claimed.
3. Hosting e2e scaffold MAY probe default cohost discoverability; **MUST** STRICT-fail until PR/issue host receipt.
4. Cross-link residual register + remaining-implement-wave + README discovery.
5. No origin push; Spec-first commits; English-only; single `arks` binary; no product `git`/`gh` spawn.

## Docker / verify slots

| Concern | Slot | Honesty |
|---------|------|---------|
| Hosting / forge-in-serve | `dod-self-host` → `run-self-host-e2e.sh` | Scaffold / Partial; Claimed only after §7 + receipt |
| Hub flat / personality | `dod-hub-matrix` / `dod-cli-matrix` | Symmetry pillars — not B-gate substitutes |
| Daemon flags | `dod-daemon-matrix` | Partial flags OK; B5+ Refuse; archive = IW-B5 slot |

## Non-goals

- Invent Claimed/PASS for hosting e2e, flat leaves, personality drop-in, double-fork, or syslog facility.
- Reopen IW-B5 / B7 / SSH-label Claimed rows.
- Product OpenSSH embed; Windows SIGHUP; OUT `extension`/`copilot` graduation.
- Treating Forgejo/Gitea as pin oracles for arks wire bytes (architecture patterns only).

## Verification (this research)

1. URLs above reviewed; git-daemon detach/syslog facts cited from docs + source/history.
2. Linked from [[sot:root/arks-partial-residual-wave]] + residual register.
3. No product Claimed language added beyond existing honesty.
