# G18 bugsim notes

1. Authoritative venue is Docker compose service `g18` (`verify/scripts/run-g18.sh`).
2. Prefer release `arks` under `CARGO_TARGET_DIR` before debug.
3. Receipt accumulation requires `--test-threads=1`.
4. Lease ≠ skip Git locks; L3 refuse-closed; no CELIR; no gix.
