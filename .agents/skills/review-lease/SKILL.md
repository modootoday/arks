---
name: review-lease
description: >
  Use when reviewing lease / concurrency / locking product work against Git's
  documented ref and index locking only: flag invented concurrency semantics and
  keep lease product design in its own SoT when it exists.
---

# Review lease / concurrency

## Focus

1. Do not invent lease product semantics inside Git-compat SoTs.
2. Respect what Git already documents for ref locking / index locking; cite primaries for those only.
3. Performance concurrency MUST preserve object IDs, pack/index/ref meanings, and negotiated wire results unless a versioned capability explicitly changes them.
4. Lease product design is owned by `.spec/sot/lease-concurrency-intent.sot.md` (rule `50-celir-boundary`). L3 Claimed shape is **M23** (lease + batch-apply + worktree/ref-shard); shared-index N-way remains OUT.

## Method

Diff → concurrency/locking claims → **documented Git lock vs invented lease product: which?**

Owning SoT: `.spec/sot/lease-concurrency-intent.sot.md`.
