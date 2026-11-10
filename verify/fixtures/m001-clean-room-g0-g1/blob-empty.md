# Fixture intent: empty blob (SHA-1)

## Authority

- [gitformat-loose](https://git-scm.com/docs/gitformat-loose) — loose object content is zlib-compressed on disk; **object id hashes the uncompressed** stream.
- In-repo summary: `.spec/research/20261108071913-git-objects-pack.research.md`.

## Uncompressed bytes (identity input)

Git hashes the following **uncompressed** byte sequence (not the zlib file body):

```text
type SP decimal-size NUL payload
```

For an empty blob:

| Field | Value |
|-------|-------|
| `type` | `blob` |
| SP | ASCII space `0x20` |
| decimal-size | `0` |
| NUL | `0x00` |
| payload | (empty) |

Concatenated literal: `blob 0` + NUL → 7 bytes: `62 6c 6f 62 20 30 00`.

## Expected OID (SHA-1, lowercase hex)

```text
e69de29bb2d1d6434b8b29ae775ad8c2e48c5391
```

## Verification intent

- `arks-oid`: parse/format round-trip for this hex (no hashing in crate).
- `arks-hash` (later): digest of the 7-byte header must equal these 20 bytes.
- Clean-room (later): `git hash-object -t blob /dev/null` equivalent on stock Git.
