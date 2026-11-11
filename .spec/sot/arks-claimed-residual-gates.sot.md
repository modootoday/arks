---
id: sot:root/arks-claimed-residual-gates
title: Claimed residual gates (post-Partial wiring)
status: active
created: 2026-11-10T22:46:00Z
updated: 2026-11-11T00:08:27Z
tags: [claimed, residual, design, docker, clean-room, ssh, openssh, tls, hub, agent, daemon, archive, syslog, rebind, no-invent-claimed, partial]
verify: manual
rel:
  depends_on:
    - research:root/20261111090827
    - research:root/20261111084400
    - research:root/20261111074523
    - research:root/20261111072342
    - sot:root/arks-remaining-implement-wave
    - sot:root/arks-self-host-server
    - sot:root/arks-server
    - sot:root/arks-hub-product-intent
    - sot:root/arks-agent-product-intent
    - sot:root/arks-agent-llm-providers
    - sot:root/arks-daemon-product-intent
    - sot:root/arks-daemon-git-command-matrix
    - sot:root/arks-cli-personality-factory
    - sot:root/verify-cleanroom-symmetry-matrix
    - sot:root/verify-directory-conventions
    - sot:root/product-verification-clean-room
    - sot:root/production-residual-register
    - sot:root/public-surface
    - sot:root/language-english-only
  evidence:
    - evidence:root/20261110231959-claimed-agent-llm
    - evidence:root/20261110233652-claimed-tls
    - evidence:root/20261110233655-claimed-hub-https
    - evidence:root/20261110233659-claimed-rebind
    - evidence:root/20261110235116-claimed-ssh
    - evidence:root/20261110233921-claimed-ssh
  governs:
    - verify/scripts/run-claimed-residual-dod.sh
    - verify/scripts/run-claimed-ssh-e2e.sh
    - verify/scripts/run-claimed-hub-https-e2e.sh
    - verify/scripts/run-claimed-agent-llm-e2e.sh
    - verify/scripts/run-claimed-daemon-archive-e2e.sh
    - verify/scripts/run-claimed-rebind-e2e.sh
    - verify/scripts/run-claimed-tls-e2e.sh
---

# Claimed residual gates (post-Partial wiring)

Document revision **0.1.4** (SoT edit counter only — **not** a product/crate release). Normative **design** for the remaining **Claimed** acceptance rows after tip Partial wiring ([[research:root/20261111072342]], tip ≥ `4f87ad8`). Audit tip ≥ `bb782db` demoted invented B2; **B2 Claimed** restored only with [[evidence:root/20261110235116-claimed-ssh]] (tip ≥ `f6c5780` + WIP). **Future implement order** (B5 reopen, B7 SSH/forge, CLI Partial stubs) is gated by [[sot:root/arks-remaining-implement-wave]] + [[research:root/20261111090827]] — **do not implement until those are cited**.

**Honesty:** **Design intent ≠ Claimed.** Scaffold Docker harness greens **MUST NOT** invent PASS/Claimed. Product paths **MUST NOT** spawn `git` / `gh` / `ssh` / `ollama`. Windows host PowerShell is **not** normative DoD — Linux Docker `arks-verify-m1:0.1.0` is the judge ([[sot:root/product-verification-clean-room]]).

## Rule

### Scope (normative IDs)

| ID | Gate | Status now | Owning product SoT(s) |
|----|------|------------|------------------------|
| **B1** | OpenSSH-exact cipher / binary-packet wire | **Claimed** (Docker stock OpenSSH ChaCha KEX/NEWKEYS/SERVICE_ACCEPT; [[evidence:root/20261110235116-claimed-ssh]]) | [[sot:root/arks-self-host-server]], [[sot:root/arks-server]] |
| **B2** | Claimed SSH git e2e | **Claimed** (Docker stock `git clone ssh://` exit 0 + archived README; [[evidence:root/20261110235116-claimed-ssh]]) | [[sot:root/arks-self-host-server]] |
| **B3** | Claimed hub HTTPS forge remote e2e | **Claimed** (Docker forge HTTPS + hub client; [[evidence:root/20261110233655-claimed-hub-https]]) | [[sot:root/arks-hub-product-intent]] |
| **B4** | Claimed LLM chatbot / review e2e | **Claimed** (Docker fixture e2e; [[evidence:root/20261110231959-claimed-agent-llm]]) | [[sot:root/arks-agent-product-intent]], [[sot:root/arks-agent-llm-providers]] |
| **B5** | Claimed daemon archive format parity | **Claimed** (Docker `cmp -s` vs stock `git-daemon` pin **2.39.5**; [[evidence:root/20261111091600-claimed-daemon-archive]]; IW-B5 via [[sot:root/arks-remaining-implement-wave]] + [[research:root/20261111090827]] / [[research:root/20261111084400]]) | [[sot:root/arks-daemon-product-intent]], [[sot:root/arks-daemon-git-command-matrix]] |
| **B5+** | Claimed syslog facility / priority parity | **Refuse (terminal)** — Partial `openlog`/`LOG_DAEMON` ≠ facility/priority Claimed; Windows Event Log ≠ syslog; reopen only with deliberate collector receipt wave | [[sot:root/arks-daemon-product-intent]] |
| **B7** | Claimed zero-downtime hot-rebind | **Claimed for HTTP** ([[evidence:root/20261110233659-claimed-rebind]] / [[evidence:root/20261111092000-claimed-rebind-http]]); **SSH Claimed** ([[evidence:root/20261111092000-claimed-rebind-ssh]]); **forge Claimed** ([[evidence:root/20261111092000-claimed-rebind-forge]]) via IW-B7 | [[sot:root/arks-self-host-server]] §17 |
| **B9** | Product in-process TLS terminate | **Claimed** (Docker rustls PEM terminate + stock curl; [[evidence:root/20261110233652-claimed-tls]]) | [[sot:root/arks-self-host-server]] ports/TLS |

**Explicitly out of Claimed graduation here:**

| ID | Disposition |
|----|-------------|
| **B5+** | **Refuse (terminal)** this wave — see scope row; residual register closed for facility Claimed |
| **B6** | Durable SSH listen + Partial live rebind — **Partial landed**; not reopened as Missing |
| **B8** | Windows SIGHUP — **N/A** (OS); file-mtime reload is Partial substitute |

### Partial vs Claimed gates (ABSOLUTE)

1. **Partial smokes stay Partial.** Examples: `arks serve ssh-smoke`, durable SSH accept labeled Partial, hub `smoke --local-tls`, agent `review --smoke [--mock]`, daemon archive path-filter tests, Partial serve-config rebind.
2. **Claimed** requires: (a) this SoT row acceptance criteria met, (b) clean-room receipt under `.spec/evidence/`, (c) owning product SoT honesty amend, (d) residual register next-queue update — **all four**.
3. **`ARKS_DOD_STRICT=0` (default):** compose services MAY exit 0 with explicit `SKIP/scaffold`.
4. **`ARKS_DOD_STRICT=1`:** unfinished Claimed rows **MUST** fail closed (exit non-zero) — **no invent PASS**.
5. Personality / serve≠hub / ports normative-vs-demo rules remain ([[sot:root/arks-cli-personality-factory]], [[sot:root/arks-self-host-server]]); this SoT does not redefine them.

### Acceptance criteria (design → Claimed)

#### B1 — OpenSSH-exact cipher / wire

- Live **stock OpenSSH client** inside clean-room completes KEX + AEAD with in-process `arks serve` SSH (no ops SSHD).
- Packet path matches OpenSSH ChaCha/AEAD expectations (length encryption / nonce / MAC-less OpenSSH ChaCha variant as applicable).
- Receipt records cipher negotiated + tip SHA + image pin.
- **Refuse:** Partial selfcheck-only; advertising `aes256-gcm@openssh.com` without selected interop.

#### B2 — Claimed SSH git e2e

- Depends on **B1**.
- Stock clean-room `git` clone and at least one of fetch/push over `ssh://` / SCP-like URL to durable Partial-or-Claimed `arks serve` SSH listener.
- Product binary never spawns `ssh`; OpenSSH is oracle only.
- **Refuse:** `ssh-smoke --git-pack` Partial alone; ops SSHD + stdio pack (that is G30-class, not in-process serve SSH Claimed).

#### B3 — Claimed hub HTTPS forge remote e2e

- `arks` forge client (flat leaves and/or transitional `arks hub`) completes authenticated HTTPS forge remote journeys against a clean-room HTTPS base.
- No `gh` subprocess.
- Asset-upload / CI-execute Claimed only when separately evidenced; default B3 bar = forge remote HTTPS happy path + receipt.
- **Refuse:** hub smoke HTTPS / `--local-tls` Partial; OUT `extension`/`copilot` refuse smokes.

#### B4 — Claimed LLM chatbot / review e2e

- Multi-turn chatbot path + review path against an openai-compat **fixture** daemon in Docker (native HTTP only).
- Exit/output honesty per agent SoTs; never invent PASS/Claimed in CEV review output.
- **Refuse:** `provider smoke` / `review --smoke` one-shot Partial.

#### B5 — Claimed daemon archive parity

- **Claimed (graduated):** byte-identical ustar stream of `upload-archive` vs stock `git-daemon` pin **2.39.5** in clean-room (`cmp -s`); receipt [[evidence:root/20261111091600-claimed-daemon-archive]]. Oracle encoder is git **v2.39.5** `archive-tar.c` `prepare_header` (`"%07o"` / `"%011o"` NUL-terminated fields, including device major/minor) — **not** a generic tar crate dialect ([[research:root/20261111084400]], pin raw https://raw.githubusercontent.com/git/git/v2.39.5/archive-tar.c). **Do not** adopt GNU-style chksum (`%06o` + NUL + SP) against this pin ([[research:root/20261111090827]]).
- **Partial (historical / still useful):** protocol path + path-filter wiring + structural member list / type / mode / payload semantic agreement — retained as the weaker oracle; structural-alone **MUST NOT** invent Claimed.
- **Closed mismatch class (Docker 501009):** prior ustar **header encoding** drift (chksum@151 + device ASCII0 vs NUL) closed by matching `prepare_header` in `arks-server` `write_ustar_entry` / `write_octal` — evidenced by `cmp -s` in `dod-claimed-daemon-archive`.
- **Refuse:** expanded protocol-body unit tests alone; semantic-only greens marketed as Claimed; product spawn of `git archive`.

#### B5+ — Claimed syslog facility parity

- **Terminal close (this goal):** **Refuse** — unix Partial `openlog`/`LOG_DAEMON` arming + stderr mirror is **not** Claimed facility/priority parity; no collector receipt exists; Windows Event Log ≠ POSIX syslog (documented N/A for B5+ Claimed).
- **Reopen bar (future wave only):** facility/priority evidenced vs stock git-daemon on unix clean-room with Docker receipt + owning daemon SoT amend — **no invent Claimed** from Partial arming alone.
- **Refuse (normative):** Partial `openlog`/`syslog` mirror-to-stderr alone.

#### B7 — Claimed zero-downtime rebind

- Serve-config reload proves **connection-drain** and/or **dual-bind** so in-flight peers are not hard-cut; receipt shows concurrent success across rebind.
- **HTTP:** **Claimed** ([[evidence:root/20261110233659-claimed-rebind]], reconfirm [[evidence:root/20261111092000-claimed-rebind-http]]).
- **SSH:** **Claimed** ([[evidence:root/20261111092000-claimed-rebind-ssh]]) — IW-B7-SSH dual-bind drain; separate from HTTP.
- **Forge:** **Claimed** ([[evidence:root/20261111092000-claimed-rebind-forge]]) — IW-B7-FORGE dual-bind drain; ≠ B3 client remote.
- `SO_REUSEPORT`-style dual-bind without drain evidence remains **Refuse as Claimed** ([[research:root/20261111090827]]).
- **Refuse:** inventing SSH/forge Claimed from HTTP green alone; hard-cut rebind without held+fresh overlap receipt.

#### B9 — Product in-process TLS

- `arks serve` (or documented product HTTPS listener) terminates TLS in-process with operator-supplied certs; stock/`arks` clients succeed over HTTPS without requiring an external reverse-proxy for the Claimed scenario.
- Ops reverse-proxy remains a **valid** deploy pattern and does **not** by itself satisfy B9.
- **Refuse:** hub client `--local-tls` smoke; docs that only describe ops TLS front.

### Docker verify slots (normative names)

New residual slots register per [[sot:root/verify-directory-conventions]] §7 (`dod-claimed-<concern>` → `run-claimed-<concern>-e2e.sh` → umbrella `dod-claimed-residual`; default SKIP / `ARKS_DOD_STRICT=1` fail-closed; **no invent Claimed**).

| Compose service | Script | Gates |
|-----------------|--------|-------|
| `dod-claimed-ssh` | `verify/scripts/run-claimed-ssh-e2e.sh` | B1, B2 |
| `dod-claimed-hub-https` | `verify/scripts/run-claimed-hub-https-e2e.sh` | B3 |
| `dod-claimed-agent-llm` | `verify/scripts/run-claimed-agent-llm-e2e.sh` | B4 |
| `dod-claimed-daemon-archive` | `verify/scripts/run-claimed-daemon-archive-e2e.sh` | B5, B5+ |
| `dod-claimed-rebind` | `verify/scripts/run-claimed-rebind-e2e.sh` | B7 |
| `dod-claimed-tls` | `verify/scripts/run-claimed-tls-e2e.sh` | B9 |
| `dod-claimed-residual` | `verify/scripts/run-claimed-residual-dod.sh` | all of the above |

```bash
docker compose -f verify/docker-compose.yml run --rm dod-claimed-residual
docker compose -f verify/docker-compose.yml run --rm dod-claimed-ssh
# ARKS_DOD_STRICT=1 fails closed until Claimed
```

### Relationship (not a fourth symmetry pillar)

1. Three peer-symmetry pillars stay under [[sot:root/verify-cleanroom-symmetry-matrix]] (`dod-cli-matrix` / `dod-daemon-matrix` / `dod-hub-matrix` / `dod-symmetry`).
2. Related hosting scaffold `dod-self-host` remains LAN-shaped umbrella smoke — complementary, not a substitute for B1/B2/B7/B9 Claimed bars.
3. This SoT is **adjacent deepen** for Claimed residuals after Partial wiring — **MUST NOT** be marketed as a fourth public-story symmetry DoD pillar.
4. Personality factory ([[sot:root/arks-cli-personality-factory]]) routes argv; Claimed depth stays here + owning product SoTs.

### Non-goals

1. Invent Claimed/PASS from design SoT, research, or SKIP harness.
2. Embed OpenSSH / libssh / russh in product crates.
3. Graduate OUT hub leaves (`extension` / `copilot`) via these gates.
4. Reopen B8 as implementable SIGHUP on Windows.
5. Treat host PowerShell greens as clean-room DoD.

## Why

Partial wiring unlocked honest progress without pretending OpenSSH-exact, forge HTTPS e2e, LLM e2e, archive parity, zero-downtime, or in-process TLS were Claimed. Operators and agents need a single design SoT that names acceptance criteria and Docker slots so the next implement waves stay Spec-first and fail-closed.

## Verification

1. Research [[research:root/20261111074523]] + this SoT linked from residual register + verify umbrella + `.spec/README.md`.
2. Compose services and SKIP/scaffold scripts exist; default `dod-claimed-residual` exits 0 with SKIP language; strict fails closed.
3. Public `docs/` / README MAY cite residual Claimed / Partial / Refuse rows only as this SoT + listed evidence IDs name — **no invent Claimed** beyond Spec (docs honesty sync cites B1/B2/B3/B4/B7(HTTP)/B9 Claimed; B5 Partial; B5+ Refuse).
4. Cross-OS honesty: Windows drives compose; Linux-in-container bash is authoritative.
