---
id: research:root/20261108170644
title: Naming final shortlist
status: active
created: 2026-11-08T17:06:44Z
updated: 2026-11-08T08:15:00Z
tags: [naming, brand, cli, crates, vcs-tone, warehouse-homage, collision]
rel:
  depends_on: [sot:root/git-compat-surface]
---

# Naming final shortlist

## Question

What is the final candidate set for the product CLI/brand name: warehouse-homage vs VCS-native short coinages vs soft `yar*` survivors — given rejects, crates availability, and serious Git-compatible tooling tone?

## Method

1. Merge three research tracks: warehouse homage, VCS-tone scoring of `yarx`/`yarq` (Y≠YOLO), and crates-free short coinages.
2. Benchmark tone against public VCS brands (git, hg/mercurial, svn, bzr, fossil, pijul, jj/jujutsu, sapling, gix) — morphology only; no invented Git wire semantics.
3. Recheck exact crate names on crates.io (HEAD) on **2026-11-08**; FREE = HTTP 404.
4. Exclude prior rejects and meme mashups from the main shortlist.
5. Deep collision pass (product/CLI brand, not crates-only): demote names that collide with active public products even when exact crates.io names are free.
6. EN/KR hearing + product-collision pass on classical “ark/arx” forms: demote names that resolve to Noah’s ark / Arc browser before warehouse.

Locks: no literal `git` substring in the name; tagline form `{Name} for Git` OK; serious VCS tooling tone (not meme-first).

## Findings

### Excluded (not main shortlist)

Rejects: `gir`, `ri`, `glean`, `arca`, `far`, `fazz`, `gix`, `gos`, `goos`, `giy`, `yarp`.  
Joke-only (appendix): meme mashups (`gyolo67`, etc.).  
Failed VCS-tone / collision bar: **`yarx`** (YARA/YARP bleed; existing security tooling noise).  
Product/CLI collision (crates FREE but **avoid as VCS product/CLI name**): **`revault`** — Bitcoin Revault + active reVault ecosystem (`revault_cli`, many `revault_*` crates). Exact crates.io `revault` / `revalut` returned FREE (404) on 2026-11-08; that does **not** clear the brand collision. Demoted from Top 7 / Lane A caps on owner confirmation.  
Hearing / scenery collision (crates FREE but **avoid as VCS product/CLI name**): **`arks`** — EN first-hear is Noah’s ark; Roman “arx/arks” scenery only weakly via Korean phonetic “areukeuseu”; also Arc browser bleed. Demoted from Top 7 / Lane B caps after arks evaluation (2026-11-08). Do **not** refill the slot with `arx` / `arcs` / `ark` (same family, taken/confused). Prefer classical warehouse Latin: **`horreum`** (preferred) or **`cellarium`**.

### Lane A — Warehouse homage (crates FREE)

| Name | Notes |
|------|--------|
| **sokko** | Primary homage candidate; compact storage feel |
| **hokanko** | JP warehouse echo; longer CLI |
| **kuraya** | Storehouse metaphor; softer than sokko |
| **horreum** | Latin warehouse; scholarly / serious — preferred classical refill after `arks` exit |
| **cellarium** | Latin cellar/store; backup classical warehouse if horreum rejected |

Notable runners (not lane caps): `kurio`, `speicher`, `kakunoko`, `sklad`, `histokura`.  
Demoted from lane caps: `revault` (see Excluded).

### Lane B — VCS-native short coinages (crates FREE)

| Name | Notes |
|------|--------|
| **pijl** | Soft coined; sits near pijul family; NL “arrow” → refs |
| **cedr** | Cedar → sapling/wood durability |
| **knar** | Wood knot / grain = history in the tree |
| **bryn** | Soft geography coinage (pijul-adjacent calm) |

Demoted from lane caps: **`arks`** (Noah’s ark / Arc browser; see Excluded).  
Filtered out of lane B (free but weak VCS tone): `vire`, `revn`, `fora`, `briq`, `gnar` (meme “gnarly”), `rexo`, `fosl` (too fossil-derivative), `rixa`/`riqx`.  
Not for refill: `arx` / `arcs` / `ark` (same collision family as `arks`).

### Lane C — Soft Y survivors (thin)

| Name | Notes |
|------|--------|
| **yorn** | Soft Y; yore/past echo without `yar*` — in Top 7 after `revault` exit |
| **yorl** | Same family; slightly less wordlike |
| **yarq** | Conditional only — registries quiet, but `yar*` still YARP-adjacent |

**`yarx` excluded** from shortlist.

### Top 7 overall

Renumbered **without `revault`** then **without `arks`** (collision / hearing demotions). Judgment: keep homage + VCS-native spine; fill the vacated short-storage slot with classical warehouse **horreum** (preferred) rather than another `ark*` form; **kuraya** remains the soft-homage alt if horreum feels too Latin/long.

1. **sokko** — strongest homage identity  
2. **pijl** — best VCS-native short  
3. **cedr** — sapling/wood family without copying sapling  
4. **hokanko** — homage backup if sokko feels too sharp  
5. **knar** — distinctive wood-history coinage  
6. **yorn** — soft Y survivor (Lane C)  
7. **horreum** — classical warehouse refill after `arks` exit; alt if closing Latin: **kuraya** (or **cellarium** if keeping Latin but rejecting horreum)

### Decision guide

- Want **cultural/storage metaphor brand** → Lane A (`sokko` / `hokanko` / `kuraya` / `horreum`); do not use `revault`; do not use `arks`/`arx`/`arcs`/`ark`.
- Want **jj/pijul/fossil calm CLI** → Lane B (`pijl` / `cedr` / `knar`); classical Latin warehouse lives in Lane A (`horreum` ≫ `cellarium`), not as an `ark*` coinage.
- Want **keep a soft Y** after rejecting yarp → Lane C (`yorn` ≫ `yarq`); do not revive `yarx`.
- Filling the old `arks` slot → **horreum** preferred; **kuraya** if homage-soft; **cellarium** only if Latin warehouse without horreum.

### Owner lock (2026-11-08)

Owner re-affirmed **`arks`** despite the hearing/collision demotion above. Normative lock: [[sot:root/product-naming]] — product / crate / bin `arks`, tagline `arks for Git`, GitHub `modootoday/arks`. Shortlist rows above remain historical collision notes, not active candidates.

### Open questions for product owner

1. ~~Homage-first vs sober VCS-first?~~ → Locked: **arks** (VCS product name; warehouse homage retired for this lock).
2. Accept longer CLI (`hokanko`, `kuraya`, `horreum`) vs 4-letter punch (`pijl`, `sokko`, `knar`, `cedr`, `yorn`)? → N/A while `arks` is locked.
3. Keep any Y-start at all (`yorn`), or swap Top-7 #6 for **kuraya** / **bryn** and close Lane C? → N/A while `arks` is locked.
4. Lock Top-7 #7 on **horreum**, or prefer **kuraya** / **cellarium**? → N/A while `arks` is locked.

## Residual warnings

- crates.io FREE is a snapshot, not a reservation; npm/PyPI/GitHub trademarks not fully re-audited for every homage name in this pass.
- Exact-crate FREE ≠ brand-safe: `revault` (Bitcoin Revault / reVault) and `arks` (Noah’s ark / Arc browser) are worked examples.
- Domain / trademark counsel still needed before lock.

## Sources

- crates.io exact-name HEAD checks (2026-11-08 session); `revault` / `revalut` FREE (404) but product collision noted separately.
- Public collision signals: Bitcoin Revault; active reVault crates (`revault_cli`, `revault_*` family).
- Public VCS brand morphology (git-scm ecosystem names as known public products).
- Prior session tracks: warehouse homage list; `yarx`/`yarq` registry + GitHub noise (npm `yarx` scaffold; PyPI `yarx` YARA placeholder; zema1/yarx).
- Owner confirmation (2026-11-08): demote/exclude `revault` from Top 7 for VCS product/CLI naming.
- Arks evaluation (2026-11-08): demote `arks` from Top 7 — EN hears Noah’s ark first; Korean phonetic “areukeuseu” only weak Roman-scenery path; Arc browser confusion; refill with `horreum` (preferred) or `cellarium`, not `arx`/`arcs`/`ark`.

- Owner lock (2026-11-08): re-affirm `arks` despite demotion; see [[sot:root/product-naming]].
