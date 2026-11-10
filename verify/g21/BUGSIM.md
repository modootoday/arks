# G21 bug simulation notes

1. `verify/scripts/run-g21.sh` MUST be LF-only. CRLF yields `set: pipefail` / `$'\r'` failures.
2. Sy-1 `add --chmod=+x` must update index mode bits; stock `git ls-files --stage` is the oracle.
3. `status -b --porcelain` must emit `## <branch>` before XY lines.
4. `switch --detach` must write a direct HEAD OID (not leave a symbolic branch).
5. Host greens are advisory; Docker `g21` is authoritative.
