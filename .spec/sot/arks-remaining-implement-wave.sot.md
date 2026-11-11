---
id: sot:root/arks-remaining-implement-wave
title: Remaining implement wave — design SoT (research-gated)
status: active
created: 2026-11-11T00:08:27Z
updated: 2026-11-11T00:08:27Z
tags: [design, residual, implement-wave, b5, b7, cli-stubs, personality, forge-in-serve, daemon, ssh, rebind, no-invent-claimed, docker, spec-first]
verify: manual
rel:
  depends_on:
    - research:root/20261111090827
    - research:root/20261111084400
    - research:root/20261111074523
    - research:root/20261111072342
    - sot:root/arks-claimed-residual-gates
    - sot:root/production-residual-register
    - sot:root/verify-directory-conventions
    - sot:root/verify-cleanroom-symmetry-matrix
    - sot:root/arks-self-host-server
    - sot:root/arks-cli-personality-factory
    - sot:root/arks-cli-output-exit-contract
    - sot:root/arks-hub-product-intent
    - sot:root/arks-daemon-product-intent
    - sot:root/arks-daemon-git-command-matrix
    - sot:root/product-verification-clean-room
    - sot:root/language-english-only
  governs:
    - verify/scripts/run-claimed-daemon-archive-e2e.sh
    - verify/scripts/run-claimed-rebind-e2e.sh
---

# Remaining implement wave — design SoT (research-gated)

Document revision **0.1.0** (SoT edit counter only — **not** a product/crate release). English-only.

**This SoT gates the remaining implement wave.** Tip surveyed after canon **`8775cc4`** / design tip **`6dae03d`**. **Design ≠ Claimed.** Product/crate changes for each IW-* row require:

1. This SoT row for the item cited in the Spec-first commit message / plan, **and**
2. Supporting research cited ([[research:root/20261111090827]] and, for B5, [[research:root/20261111084400]]), **and**
3. Docker verify slot + Partial vs Claimed bar below understood (STRICT fail-closed; no invent PASS).

**IW-B5** graduated **Claimed** with [[evidence:root/20261111091600-claimed-daemon-archive]]. Remaining rows stay Partial until their Claimed bars + receipts land.

## Rule

### Absolute honesty

1. Partial smokes, SKIP harness greens, host PowerShell, and semantic tar equality **MUST NOT** invent Claimed.
2. Product paths **MUST NOT** spawn `git` / `gh` / `ssh` / `ollama`.
3. Clean-room Linux Docker (`arks-verify-m1` + Git pin **2.39.5**) is normative DoD ([[sot:root/product-verification-clean-room]]).
4. New Claimed slots register per [[sot:root/verify-directory-conventions]] §7 — Spec first, then harness.
5. This wave is **adjacent** to the three symmetry pillars ([[sot:root/verify-cleanroom-symmetry-matrix]]) — **not** a fourth pillar.

### Ordered implementation backlog (later waves only)

Order is **Spec → research cite → product → Docker receipt → SoT Claimed amend**. Do not reorder to chase green SKIP.

| Order | ID | Surface | Status now | Partial bar (honest) | Claimed bar | Docker slot |
|------:|----|---------|------------|----------------------|-------------|-------------|
| 1 | **IW-B5** | Daemon `upload-archive` ustar | **Claimed** ([[evidence:root/20261111091600-claimed-daemon-archive]]) | Protocol + structural/semantic member equality | **Byte-identical** ustar vs stock `git-daemon` pin **2.39.5** (`cmp -s`); encoder = git `prepare_header` (`"%07o"` / `"%011o"`, device fields written, chksum sum-with-spaces) | `dod-claimed-daemon-archive` / `run-claimed-daemon-archive-e2e.sh` |
| 2 | **IW-B7-SSH** | Serve-config SSH rebind drain | **Claimed** ([[evidence:root/20261111092000-claimed-rebind-ssh]]) | Live rebind / durable listen labeled Partial (§17(c)) | Held SSH session survives reload + new SSH peer succeeds across dual-bind/drain; receipt separate from HTTP | `dod-claimed-rebind` scenario `ssh` |
| 3 | **IW-B7-FORGE** | Serve-config forge rebind drain | **Claimed** ([[evidence:root/20261111092000-claimed-rebind-forge]]) | Forge module live rebind Partial | Held forge HTTP(S) session + new peer across reload; ≠ B3 client remote Claimed | `dod-claimed-rebind` scenario `forge` |
| 4 | **IW-SSH-LABEL** | Serve SSH process-label / readiness | **Claimed** (B1/B2 + IW-B7-SSH) | Durable listen **must** say Partial while rebind Claimed absent | Operator surfaces may drop Partial readiness label only when B1/B2 **and** IW-B7-SSH Claimed (or SoT explicitly splits wire Claimed vs listen Claimed) | Evidence + `arks-self-host-server` honesty amend |
| 5 | **IW-FORGE-SERVE** | Forge-in-serve hosting depth | **Partial** (scaffold deepened: default cohost) | Local forge API under `arks serve` even when SSH enabled (no silent SSH-only cut) | Hosting e2e forge module Claimed (PR/issue host journeys); **B3** alone ≠ this row | Future `dod-claimed-*` only after §7 register |
| 6 | **IW-HUB-FLAT** | Hub flat-leaf depth | **Partial** (+ **B3 Claimed**) | Flat leaves + hub serve forwards remain Partial stubs; B3 HTTPS remote happy path Claimed | Peer-faithful leaf bodies + hub matrix STRICT greens | Symmetry hub pillar / hub matrices — not invent from B3 |
| 7 | **IW-PERSONALITY** | Personality / Registry / ExitCodePolicy | **Partial** (skeleton landed) | Resolver + Registry + OverlapFacade + ExitCodePolicy OR-of-failure unit-tested | Peer-native stdout/exit drop-in evidence for `git`/`gh` personalities | CLI symmetry matrices + personality SoT amend |
| 8 | **IW-DAEMON-FLAGS** | Daemon deepen (non-B5) | **Partial** / **B5+ Refuse** (terminal for Claimed) | path-filter, `--syslog` openlog, `--user`/`--group`, detach Partial landed | **No** Claimed for classic double-fork or syslog facility (B5+ **Refuse**); archive Claimed is IW-B5 only | daemon matrix / archive slot as applicable |

**B7 HTTP** remains **Claimed** ([[evidence:root/20261110233659-claimed-rebind]]) and is **not** reopened by this SoT. **B1/B2/B3/B4/B9** remain Claimed; do not re-litigate.

**Explicit non-items (do not schedule as implement):** B5+ syslog facility Claimed (**Refuse**); B8 Windows SIGHUP (**N/A**); OUT `extension`/`copilot` graduation; product OpenSSH embed; L9 / live G8 invent PASS.

### Partial vs Claimed (per-class detail)

#### IW-B5 — ustar Claimed reopen

- **Cite:** [[research:root/20261111090827]], [[research:root/20261111084400]], pin source https://raw.githubusercontent.com/git/git/v2.39.5/archive-tar.c
- **Status:** **Claimed** — Docker `cmp -s` + [[evidence:root/20261111091600-claimed-daemon-archive]].
- **Partial (historical):** path-filter + structural/semantic agreement remains a weaker oracle.
- **Claimed bar (met):** `cmp -s` stock vs arks full stream in Docker; unit tests for first-header / device / chksum fields.
- **Refuse as Claimed:** tarfile-only greens; GNU chksum folklore (`%06o`+NUL+SP) against pin; host Git ≠ 2.39.5; product `git archive` spawn.
- **Closed mismatch class (501009):** header encoding (chksum digit drift; device ASCII `0` vs NUL historically) closed by `prepare_header` parity.

#### IW-B7-SSH / IW-B7-FORGE

- **Cite:** [[research:root/20261111090827]] (Cloudflare ecdysis / GitHub multibinder / Teleport — `SO_REUSEPORT` orphan risk).
- **Implement path:** same-process **bounded dual-bind + drain** mirroring HTTP B7 Claimed (`serve.rs`); SSH sessions spawned on threads so held peers do not block accept/rebind.
- **Partial (historical):** hard-cut live rebind without held-peer drain receipt.
- **Claimed:** receipt shows concurrent success across reload (held old + fresh new + fresh old during drain); separate evidence IDs — [[evidence:root/20261111092000-claimed-rebind-ssh]], [[evidence:root/20261111092000-claimed-rebind-forge]].
- **Refuse as Claimed:** HTTP B7 receipt reused for SSH/forge; dual-bind without drain evidence.

#### IW-SSH-LABEL

- Wire Claimed (B1/B2) **does not** automatically Claim process readiness language.
- **Claimed (this wave):** with B1/B2 **and** IW-B7-SSH Claimed, durable listen operator surfaces **MAY** drop Partial *readiness* labeling and cite wire+rebind Claimed ([[sot:root/arks-self-host-server]] §17). Hosting e2e / forge-in-serve depth remain Partial where those rows say so.
- Historical rule retained: while rebind Claimed was absent, durable listen **MUST** keep Partial labeling.

#### IW-FORGE-SERVE / IW-HUB-FLAT / IW-PERSONALITY / IW-DAEMON-FLAGS

- Owning product SoTs remain authoritative for surface grammar.
- This SoT only locks **order**, **bars**, and **research gate** so agents do not thrash.
- **IW-FORGE-SERVE Partial deepen (this wave):** default `arks serve` cohosts HTTP + SSH + forge (no silent SSH-only cut). Hosting e2e PR/issue journeys remain **Partial** until a §7 `dod-claimed-*` receipt — **B3 ≠ this row**.
- **IW-HUB-FLAT / IW-PERSONALITY:** keep Partial honesty; do not invent Claimed from stub forwards or ExitCodePolicy unit greens alone.
- **IW-DAEMON-FLAGS:** Partial deepen closed for this wave; **B5+ Refuse** unchanged; no classic double-fork Claimed.

### Docker verify slots

| Compose / script | Gates |
|------------------|-------|
| `dod-claimed-daemon-archive` → `run-claimed-daemon-archive-e2e.sh` | IW-B5 (Partial default; Claimed on `cmp -s` + SoT amend) |
| `dod-claimed-rebind` → `run-claimed-rebind-e2e.sh` | B7 HTTP Claimed; IW-B7-SSH / IW-B7-FORGE scenarios SKIP/Partial until Claimed |
| `dod-claimed-residual` | Orchestrator — STRICT fail-closed for unfinished Claimed rows |
| Symmetry `dod-*-matrix` | IW-HUB-FLAT / IW-PERSONALITY / IW-DAEMON-FLAGS — not B-gate substitutes |

```bash
# Normative (Linux-in-container). Default SKIP/Partial OK.
docker compose -f verify/docker-compose.yml run --rm dod-claimed-daemon-archive
docker compose -f verify/docker-compose.yml run --rm dod-claimed-rebind
# ARKS_DOD_STRICT=1 fails closed until Claimed SoT rows flip.
```

### Spec-first commit contract (mandatory)

Before any product PR/commit for an IW-* row:

1. Spec amend (this SoT and/or [[sot:root/arks-claimed-residual-gates]] / owning product SoT) stating acceptance.
2. Research cite in commit body (`research:root/20261111090827` ± B5 `20261111084400`).
3. Harness scenario names match table above.
4. Claimed status flips **only** with scrubbed `.spec/evidence/` + residual register update.

### Cross-links (normative)

| Ledger | Role |
|--------|------|
| [[sot:root/arks-claimed-residual-gates]] | B1–B9 Claimed residual gates; B5 Partial; B7 HTTP Claimed |
| [[sot:root/production-residual-register]] | Post-terminal next-queue cites this SoT |
| [[sot:root/verify-directory-conventions]] | §7 registration checklist |
| [[sot:root/arks-self-host-server]] | Serve §17 Partial durable / rebind honesty |
| [[sot:root/arks-cli-personality-factory]] | Personality Partial shell |

## Why

Agents repeatedly burned tokens implementing residual Claimed rows without a frozen encoder oracle, rebind drain definition, or process-label honesty split. A single research-gated design SoT makes the next implement wave Spec-first and fail-closed.

## Verification

1. `spec check` clean for this SoT + linked residual / claimed-gates / README discovery entries.
2. No product Claimed language added from this design-only wave.
3. Public docs MAY cite IW-* as design backlog only — never as Done.
