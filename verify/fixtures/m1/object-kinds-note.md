# Future fixture note: tree and commit OIDs (non-claimed M1)

**Not claimed for M1 `arks-oid` vectors.** Listed so later milestones know identity uses the **same** uncompressed header rule; only `type`, size, and payload differ.

## Authority

- [gitformat-loose](https://git-scm.com/docs/gitformat-loose) (identity function).
- Payload shapes: plumbing docs and glossary cited in `.spec/research/20261108071913-git-objects-pack.research.md` — **do not** invent entry ordering or commit field layout here.

## Identity rule (unchanged)

For any Git object kind documented on gitformat-loose:

```text
OID = H( type SP decimal-size NUL payload )
```

where `H` is SHA-1 or SHA-256 per repository object format.

## Examples (intent only — no expected OIDs in M1)

| Kind | Illustrative uncompressed prefix | Payload source |
|------|----------------------------------|----------------|
| `tree` | `tree <n>\0` | mode SP name NUL hex-oid … (see tree plumbing) |
| `commit` | `commit <n>\0` | tree / parents / author / committer / message |

When `arks-object` claims tree/commit encode-decode, add sibling files under `verify/fixtures/m1/` with oracle hex from stock Git or clean-room — not before.
