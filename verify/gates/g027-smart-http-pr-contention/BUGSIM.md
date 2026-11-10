# G27 BUGSIM — arks smart HTTP PR contention

## Intent

Reproduce thousands-class branch / synthetic PR-ref contention **through arks smart HTTP** with concurrent stock-Git agents. Integrity must fail-closed on same-ref push races.

## Honesty

- ≠ GitHub PR UX / Checks / merge-queue / protection rules
- ≠ live forge throughput or rate-limit capacity
- SSH / `git://` / dumb HTTP / protocol v2 Remain Refuse

## Induced faults

| Case | Induce | Expect |
|------|--------|--------|
| Contended same-ref push | N agents `git push` same ref | ≤1 success; tip stock-resolvable; no torn OID |
| Heavy-gate overlap | Foreign `.run.lock.dir` present | Runner exit 75 refuse |

## Non-claims

Do not treat PASS as GitHub Actions, Checks, or merge-queue capacity evidence.
