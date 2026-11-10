# M1 fixture intent (advisory)

English-only intent documentation for Milestone 1 identity vectors. **No binary fixture bytes are required here** for `arks-oid`; this directory records how known OIDs relate to Git loose-object identity rules.

**Roadmap v0.3.1:** M1 **restart / re-verify** — fixture rows are design input; product interop claims stay **clean-room authoritative** (`verify/matrix-m1.md`, `.spec/sot/product-verification-clean-room.sot.md`).

**Primary:** [gitformat-loose](https://git-scm.com/docs/gitformat-loose) — object id is the hash of **uncompressed** `type SP size NUL` + payload (summarized in `.spec/research/20261108071913-git-objects-pack.research.md`).

**Consumers:** `crates/arks-oid` / `arks-hash` unit tests; G1 oracle rows M1-C01/C02 (`verify/g1`); Specfold unit SoTs `.spec/sot/arks-oid.sot.md`, `.spec/sot/arks-hash.sot.md`.

| Doc | Kind | Status |
|-----|------|--------|
| [blob-empty.md](./blob-empty.md) | blob (empty) | claimed M1 |
| [blob-hello.md](./blob-hello.md) | blob (`hello`) | claimed M1 |
| [object-kinds-note.md](./object-kinds-note.md) | tree / commit | future / non-claimed |
