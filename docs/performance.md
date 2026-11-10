---
title: Performance
description: Clean-room git vs arks timings — parallelism evidence, not slogans
nav_order: 60
sections:
  - read
  - bars
  - {c: chart, name: medians, type: bar, data: m18-journey-ms}
  - {c: chart, name: ratios, type: bar, horizontal: true, data: m18-ratios}
  - summary
  - docker
  - {c: cta, name: next}
---

# Performance

Side-by-side timings for stock **Git** vs **arks** come from Docker **clean-room verification** used by maintainers. Prefer reading them as **wall-clock evidence on shared journeys** that matter under parallel / multi-session load — not as a product brochure slogan, and **not** how end users run arks.

You run the **native** `arks` CLI on your machine. Docker is the verification venue only.

> [!WARNING]
> This page does **not** advertise a blanket “faster than Git” claim. Cite the receipt when you quote a number. Host laptop timings are advisory only — they are not product-verified. Absolute milliseconds change when seeds grow; **ratios** are the honest compare.

## How to read these numbers

- **Motive link:** Faster single journeys do not by themselves prove contention relief; they are receipts that the native engine stays competitive on the allow-list while concurrency work proceeds elsewhere ([Concurrent sessions](../concurrent-sessions/)).
- **Venue:** pinned clean-room image; in-image stock Git **2.39.5** vs release-profile `arks`.
- **Aggregation:** median wall-clock over 3 iterations (`median_p95`).
- **Ratio:** `arks_median / git_median`. Values below `1.0` mean arks used less wall-clock on that fixture.
- **Bars:** ≤ **0.8×** on every allow-list journey remains the broad measured bar. ≤ **0.5×** is the aggressive bar; **Clone HTTP** stays **aggressive_na** on the latest sole remeasure (needs product smart-HTTP clone work — not seed tricks).

Carrying receipts:

- [G19 remeasure at tip `c1b0b24`](https://github.com/modootoday/arks/tree/main/.spec/evidence/20261110003836-g19-remeasure-c1b0b24) — charts below
- [M18 / G19 exit (historical progressive ≤0.5× set)](https://github.com/modootoday/arks/tree/main/.spec/evidence/20261109143000-m18-pf4-3-g19-exit)
- [M17 / G19 full allow-list (19/19 ≤0.8×)](https://github.com/modootoday/arks/tree/main/.spec/evidence/20261109090000-m17-exit-g19-measured-19)
- [PF5 aggressive_na amortization notes](https://github.com/modootoday/arks/tree/main/.spec/evidence/20261111001400-fw-pf5-na-amortization-g19-7of8)
- Journey catalog notes: [verify/matrices/matrix-journey.md](https://github.com/modootoday/arks/blob/main/verify/matrices/matrix-journey.md)

## Claim bars (honesty)

| Claim | Result | Evidence |
|-------|--------|----------|
| Full journey allow-list ≤0.8× | **19/19** measured, N/A-free on latest sole remeasure | [G19 remeasure `c1b0b24`](https://github.com/modootoday/arks/tree/main/.spec/evidence/20261110003836-g19-remeasure-c1b0b24) |
| Aggressive ≤0.5× | **18** journeys ≤0.5× on that receipt; **Clone HTTP** remains **aggressive_na** | same receipt · PF5 notes above |

## Median wall-clock (ms)

*Clean-room G19 remeasure — median ms (Git vs arks). Source: `.spec/evidence/20261110003836-g19-remeasure-c1b0b24/`. Seeds differ from M18; compare with ratios.*

## Ratio (arks / Git)

*Lower ratio = less wall-clock for arks on that clean-room fixture. Clone HTTP ≈ 0.66 and stays outside the aggressive ≤0.5× claim set.*

## Summary

On tip `c1b0b24`, seed-amortized journeys moved several former mid/quantum residuals under ≤0.5× **as ratios**. Absolute ms are not comparable to the older M18 table without reading the receipt. Clone HTTP remains the honest aggressive holdout.

> [!NOTE]
> Charts track the latest sole G19 receipt on disk. When another sole compare lands under `.spec/evidence/`, update `_data/charts/` and this page together — do not invent timings.

## Docker is for verification

| Audience | How arks runs |
|----------|----------------|
| **End users / developers trying the product** | Native `arks` binary (`cargo build -p arks --release` or an installed binary on `PATH`) |
| **Maintainers proving claims** | Docker clean-room compose services (`g16` journey compare, `g19` comparative timing, and related gates) |

If a page ever sounded like “run arks in Docker,” that was verification framing, not the product runtime.

## Next

[Concurrent sessions](../concurrent-sessions/) [Limits](../limits/) [Getting started](../getting-started/)
