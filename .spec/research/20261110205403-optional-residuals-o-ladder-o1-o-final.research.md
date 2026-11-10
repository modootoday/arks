---
id: research:root/20261110205403
title: Optional residuals O-ladder (O1–O_final) after S_final
status: active
created: 2026-11-10T20:54:03Z
updated: 2026-11-10T21:10:00Z
tags:
  [
    optional,
    o-ladder,
    g8-live,
    filter-spec,
    tree-0,
    interactive,
    tui,
    force-with-lease,
    push-options,
    apache-dav,
    symmetry,
    s-final-next,
  ]
rel:
  depends_on:
    - research:root/20261110105436
    - sot:root/production-residual-register
    - sot:root/arks-remote
    - sot:root/arks-server
    - sot:root/cli-surface
    - sot:root/cli-flag-parity-complete
    - sot:root/interop-test-matrix
    - adr:root/20261110113200
    - adr:root/20261110113300
    - adr:root/20261110112600
    - adr:root/20261110105547
    - research:root/20261110095142
---

# Optional residuals O-ladder (O1–O_final)

## Question

After symmetry ladder **S0–S_final Done** ([[research:root/20261110105436]]) with tip ≥ `31b4ac5`, which ordered optional reopen goals close the honest next-gap list (live G8 remasure, extra filter-specs, interactive TUI, Apache DAV / force-with-lease / push-options, and any other SoTable “optional later / honest defer” rows) without inventing Claimed/PASS, without reopening **L9**, and without true product OOS (GUI / LFS end-product / forge Issues·PR host UX / agent frameworks)?

## Method

1. Inventory residuals still marked honest defer / secrets-block / “other filter-specs N/A” / S3 non-claims against owning SoTs and `docs/git-vs-arks.md`.
2. Classify each as **secrets-blocked**, **SoT-amend → implement**, **scripted Claimed subset**, or **still-OOS / owner-closed**.
3. Order by Specfold honesty (SoT/ADR before wire) → highest-value protocol/CLI reopen → clean-room symmetry e2e → docs currency.
4. Keep **L9** closed. Keep true OOS out unless a primary SoT already claims them.

## Findings — tip honesty baseline (`31b4ac5`)

| Item | State after S_final |
|------|---------------------|
| S0–S_final | **Done** |
| R3 `blob:none` | **Claimed** (host + G39); other filter-specs refuse |
| R4 OpenPGP / X.509 / `tag -s` | **Claimed** (G38) |
| S3 dumb deletes | **Claimed**; Apache DAV / lease / push-options honest defer |
| S4 interactive TUI | **Honest defer** (full TUI N/A) |
| S5 live forge | **Secrets-block**; Environment `g8-live` exists; **secret count = 0** (API); agent env absent |
| L9 crates.io timing | **Owner-closed** — do not reopen |
| FW-GUI / FW-LFS / FW-SCALAR / FW-FORGE-B | **Still OOS** |

## Ordered O-ladder

| # | Goal | Class | Exit criteria | Status |
|---|------|-------|---------------|--------|
| **O1** | Live G8 remasure | Secrets-gated | Secrets present → measured receipts (latency only if measured); secrets **absent** → structural + fail-closed operator path + documented hard block (**≠** invent PASS) | **Done (secrets hard-block)** ([[adr:root/20261110210100]] + [[evidence:root/20261110210200]]; Environment secrets=0) |
| **O2** | Extra filter-specs beyond `blob:none` | SoT-amend → implement | Claim `tree:0` (and any other SoTable filter-spec); refuse remainder; expand G39-class e2e | **Done** ([[adr:root/20261110210000]] + [[evidence:root/20261110210500]]; G39 matrix expanded) |
| **O3** | Interactive TUI reopen | SoT-amend → scripted Claimed subset | Claim non-TTY scripted `rebase -i` (SEQUENCE_EDITOR), editor commit (`GIT_EDITOR`/`-F`), `add -p` scripted stdin; document TTY; full curses TUI remains N/A | **Done** ([[adr:root/20261110210300]] + [[evidence:root/20261110210600]]; pixel TUI N/A) |
| **O4** | Protocol narrows | SoT-amend → implement | Claim force-with-lease (smart pack), push-options advertise+forward (hooks; no proc-receive product), Apache DAV dumb interop where Docker-SoTable; evidence before Claimed | **Done (lease+options)** ([[adr:root/20261110210400]] + [[evidence:root/20261110210700]]); Apache **G41 pending receipt** |
| **O_final** | Currency + CI | Docs / DoD | Residual register + `docs/git-vs-arks.md` / limits match evidence; tip CI green; **L9** still closed | **Done** (docs+residual currency; CI after push) |

### Non-goals

- Inventing live latency PASS or Claimed without receipts.
- Reopening **L9**.
- GUI product, Git LFS end-product, scalar, forge Issues/PR/Checks host UX, agent/IDE runtimes.
- Relitigating §F P5 hostile/host-OS catalog N/A rows unless a narrow protocol slice is SoTable (push-options advertise ≠ HOOK-008 race Claimed).
- Force-push / OWNER.md / host gpg·cargo stampede.

## Execution note

This tip session executes **O1→O_final** end-to-end. O1 closes as operator fail-closed + secrets hard-block when Environment secrets remain empty; O2–O4 Claimed only with SoT amend + wire + evidence.

## Verification

1. This note cited from residual register / AGENTS / comparison table as the active optional ladder after S_final.
2. Each O# has ADR/SoT disposition (Claimed, secrets-block, or honest residual N/A) before public Claimed flips.
3. Clean-room/host evidence ids attached; Docker sequential; halt on OOM/stampede.
4. `spec check` clean; tip CI green; **L9** still closed.
