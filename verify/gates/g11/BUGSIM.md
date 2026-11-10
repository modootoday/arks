# G11 bug simulation

1. **CRLF in `run-g11.sh`** — same class as G14/G13: author LF-only; Windows checkout may convert. Gate uses `#!/usr/bin/env bash` + `set -euo pipefail`. If `set: pipefail` appears, normalize LF.
2. **Lease skips Git locks** — forbidden. Unit `lease_held_still_honors_git_ref_lock` must stay green; do not “optimize” by bypassing `*.lock`.
3. **HighConcurrency silent default** — forbidden. Default remains `GitCompat`. HC is opt-in claimed shape (M23); shared-index N-way still OUT.
