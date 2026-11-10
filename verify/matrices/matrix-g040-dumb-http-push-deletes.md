# G40 clean-room matrix — S3 dumb HTTP push deletes symmetry twin

Unlock: [[adr:root/20261110112600]], [[evidence:root/20261110112600]] (host wire).
Primary: [git-http-push](https://git-scm.com/docs/git-http-push), [RFC 4918](https://datatracker.ietf.org/doc/html/rfc4918) DELETE.

| ID | Scenario | Gate | Automated | Expect |
|----|----------|------|-----------|--------|
| G40-C01 | Dumb clone + push topic via `arks serve-http --dumb-only` | G40 | yes | topic on bare |
| G40-C02 | `arks push origin :topic` WebDAV DELETE | G40 | yes | topic absent |
| G40-C03 | Refuse `push :main` (HEAD target) | G40 | yes | non-zero |
| G40-C04 | gix-exit scan | G40 | yes | clean |

**Non-claims:** Apache `mod_dav` peer; force-with-lease; inventing non-WebDAV delete.
