# Fixture intent: blob payload `hello` (SHA-1)

## Authority

- [gitformat-loose](https://git-scm.com/docs/gitformat-loose) — object id is hash of uncompressed `type SP size NUL` + payload.
- In-repo summary: `.spec/research/20261108071913-git-objects-pack.research.md`.

## Uncompressed bytes (identity input)

| Field | Value |
|-------|-------|
| `type` | `blob` |
| SP | `0x20` |
| decimal-size | `5` |
| NUL | `0x00` |
| payload | ASCII `hello` (`68 65 6c 6c 6f`) |

Header + payload: `blob 5` + NUL + `hello` → 12 bytes total.

## Expected OID (SHA-1, lowercase hex)

```text
b6fc4c620b67d95f953a5c1c1230aaab5db5a1b0
```

## Verification intent

- `arks-oid`: parse/format round-trip for this hex.
- `arks-hash` (later): SHA-1 of the 12-byte uncompressed sequence.
- Clean-room (later): stock Git `hash-object` on a file containing `hello` with blob type.
