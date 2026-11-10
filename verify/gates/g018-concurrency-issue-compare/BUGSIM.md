# G18 bugsim notes

1. Authoritative venue is Docker compose service `g018-concurrency-issue-compare` (`verify/scripts/run-g018-concurrency-issue-compare.sh`).
2. Prefer release `arks` under `CARGO_TARGET_DIR` before debug.
3. Receipt accumulation requires `--test-threads=1`.
4. Lease ≠ skip Git locks; L3 refuse-closed; no CELIR; no gix.
