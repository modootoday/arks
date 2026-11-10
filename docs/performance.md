---
title: Performance
description: Clean-room git vs arks timings — parallelism evidence, not slogans
sections:
  - read
  - bars
  - ratios
  - summary
  - docker
  - {c: cta, name: next}
---

# Performance

Side-by-side timings for stock **Git** vs **arks** come from Docker **clean-room verification** used by maintainers. Prefer reading them as **wall-clock evidence on shared journeys** that matter under parallel / multi-session load — not as a product brochure slogan, and **not** how end users run arks.

You run the **native** `arks` CLI on your machine. Docker is the verification venue only.

## How to read these numbers

- **Motive link:** Faster single journeys do not by themselves prove contention relief; they are receipts that the native engine stays competitive on the allow-list while concurrency work proceeds elsewhere ([Concurrent sessions](../concurrent-sessions/)).
- **Venue:** pinned clean-room image; in-image stock Git **2.39.5** vs release-profile `arks`.
- **Aggregation:** median wall-clock over 3 iterations (`median_p95`).
- **Ratio:** `arks_median / git_median`. Values below `1.0` mean arks used less wall-clock on that fixture.
- **Bars:** M17 requires every listed journey ≤ **0.8×**. M18 claims a progressive ≤ **0.5×** set on **11 of 19** journeys; PF5 aggressive follow-on is **partial 7/8** (CLONE-LOCAL remains outside that aggressive set).
- **Host laptop timings are advisory only** — they are not product-verified.
- This page does **not** advertise a blanket “faster than Git” claim. Cite the receipt when you quote a number.

Carrying receipts:

- [M18 / G19 exit (11/19 ≤0.5×)](https://github.com/modootoday/arks/tree/main/.spec/evidence/20261109143000-m18-pf4-3-g19-exit) — charts below
- [M17 / G19 full allow-list (19/19 ≤0.8×)](https://github.com/modootoday/arks/tree/main/.spec/evidence/20261109090000-m17-exit-g19-measured-19)
- [PF5 aggressive partial 7/8](https://github.com/modootoday/arks/tree/main/.spec/evidence/20261111001400-fw-pf5-na-amortization-g19-7of8)
- Journey catalog notes: [verify/matrix-journey.md](https://github.com/modootoday/arks/blob/main/verify/matrix-journey.md)

## Median wall-clock (ms)

`chart:bar`

| Journey | Git | arks |
|---------|----:|-----:|
| Local | 60 | 40 |
| Branch | 6 | 4 |
| Merge FF | 4 | 2 |
| Rebase | 21 | 8 |
| Bundle | 6 | 4 |
| Clone HTTP | 55 | 31 |
| Clone path | 37 | 12 |
| Alias | 60 | 40 |
| Worktree | 24 | 14 |
| Tag | 177 | 64 |
| Stash | 12 | 8 |
| Cherry-pick | 21 | 6 |
| Diff | 199 | 74 |
| rm / mv | 8 | 6 |
| Archive | 32 | 16 |
| Reflog | 8 | 4 |
| Revert | 6 | 2 |
| Repack | 29 | 10 |
| Submodule | 92 | 16 |

*Clean-room M18 / G19 sole run — median ms (Git vs arks)*

## Ratio (arks / Git)

`chart:bar horizontal`

| Journey | arks / Git |
|---------|-----------:|
| Local | 0.67 |
| Branch | 0.67 |
| Merge FF | 0.50 |
| Rebase | 0.38 |
| Bundle | 0.67 |
| Clone HTTP | 0.56 |
| Clone path | 0.32 |
| Alias | 0.67 |
| Worktree | 0.58 |
| Tag | 0.36 |
| Stash | 0.67 |
| Cherry-pick | 0.29 |
| Diff | 0.37 |
| rm / mv | 0.75 |
| Archive | 0.50 |
| Reflog | 0.50 |
| Revert | 0.33 |
| Repack | 0.34 |
| Submodule | 0.17 |

*Lower ratio = less wall-clock for arks on that clean-room fixture*

## Summary

| Claim | Result | Evidence |
|-------|--------|----------|
| Full journey allow-list ≤0.8× | **19/19** measured, N/A-free | [M17 receipt](https://github.com/modootoday/arks/tree/main/.spec/evidence/20261109090000-m17-exit-g19-measured-19) |
| Aggressive ≤0.5× progressive set | **11/19** at M18; PF5 follow-on **7/8** (CLONE-LOCAL remains aggressive_na) | [M18](https://github.com/modootoday/arks/tree/main/.spec/evidence/20261109143000-m18-pf4-3-g19-exit) · [PF5 7/8](https://github.com/modootoday/arks/tree/main/.spec/evidence/20261111001400-fw-pf5-na-amortization-g19-7of8) |

Journey-class residuals at M18 exit (still ≤0.8×): process-tax (`Local`, `Alias`), timer-quantum short journeys, and mid residuals (`Clone HTTP`, `Worktree`). Details stay in the receipt and maintainer residual register — not restated here as marketing.

## Docker is for verification

| Audience | How arks runs |
|----------|----------------|
| **End users / developers trying the product** | Native `arks` binary (`cargo build -p arks --release` or an installed binary on `PATH`) |
| **Maintainers proving claims** | Docker clean-room compose services (`g16` journey compare, `g19` comparative timing, and related gates) |

If a page ever sounded like “run arks in Docker,” that was verification framing, not the product runtime.

## Next

[Concurrent sessions](../concurrent-sessions/) [Limits](../limits/) [Getting started](../getting-started/)
