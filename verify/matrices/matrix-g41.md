# G41 clean-room matrix — Apache DAV dumb HTTP interop (O4)

Unlock: [[adr:root/20261110210400]], [[sot:root/arks-remote]] dumb push subset.
Primary: [git-http-push](https://git-scm.com/docs/git-http-push), [RFC 4918](https://datatracker.ietf.org/doc/html/rfc4918), stock Apache `mod_dav`.

| ID | Scenario | Gate | Automated | Expect |
|----|----------|------|-----------|--------|
| G41-C01 | `arks push` create/FF vs Apache `mod_dav` bare repo | G41 | yes | tip advances |
| G41-C02 | `arks push :<ref>` delete vs Apache DAV | G41 | yes | ref removed |
| G41-C03 | gix-exit scan | G41 | yes | clean |

**Non-claims:** full litmus suite; force-with-lease on dumb; inventing non-WebDAV delete.
