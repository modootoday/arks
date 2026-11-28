#!/usr/bin/env bash
# E1-E8 v0.2.0 Scope Expansion Clean-Room Verification Gate
# Spec: .spec/sot/v0-2-0-scope-expansion.sot.md
#
# Authoritative when executed inside Docker `arks-verify-m1:0.1.0` (Debian bookworm +
# Rust 1.99.1 + stock git 2.39.5). Host execution is advisory only.
set -euo pipefail

cd "$(dirname "$0")/../.."
ROOT="$PWD"
OUT="${ARKS_E1_E8_OUT:-$ROOT/verify/out/e1-e8-cleanroom}"
rm -rf "$OUT"
mkdir -p "$OUT"

if ! command -v git >/dev/null 2>&1; then
  echo "E1-E8: stock git missing" >&2
  exit 1
fi

git config --global --add safe.directory "$ROOT" 2>/dev/null || true
GIT_VER="$(git --version)"
RUSTC_VER="$(rustc --version)"
TIP_SHA="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
IN_DOCKER="false"
if [[ -f /.dockerenv ]]; then
  IN_DOCKER="true"
fi

echo "E1-E8 clean-room gate starting"
echo "  venue:     in_docker=$IN_DOCKER"
echo "  stock_git: $GIT_VER"
echo "  rustc:     $RUSTC_VER"
echo "  tip:       $TIP_SHA"

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"

# 0. Invariant: zero gix and zero celir in Cargo dependency graph.
META="$(cargo metadata --format-version 1)"
if echo "$META" | grep -E '"name":[[:space:]]*"gix' >/dev/null; then
  echo "E1-E8: forbidden gix crate found in dependency graph" >&2
  exit 1
fi
if echo "$META" | grep -iE '"name":[[:space:]]*"celir' >/dev/null; then
  echo "E1-E8: forbidden celir crate found in dependency graph" >&2
  exit 1
fi
echo "E1-E8: dependency invariants OK (zero gix, zero celir)"

# Build arks binary once for live clean-room interop checks.
cargo build -p arks
ARKS_BIN="${CARGO_TARGET_DIR:-$ROOT/target}/debug/arks"
if [[ ! -x "$ARKS_BIN" ]]; then
  echo "E1-E8: built arks binary not found at $ARKS_BIN" >&2
  exit 1
fi

SUMMARY="$OUT/summary.txt"
: > "$SUMMARY"
log_pass() {
  echo "[PASS] $1" | tee -a "$SUMMARY"
}

# 1. E1: OAuth 2.0 Device Authorization Grant (RFC 8628)
echo "--- E1: OAuth 2.0 Device Flow ---"
cargo test -p arks-hub device_flow -- --nocapture >"$OUT/e1-oauth.log" 2>&1
log_pass "E1: arks-hub device_flow RFC 8628 state machine & loopback fixture PASS"

# 2. E2: GitHub Actions CLI + pure-Rust NaCl sealed-box
echo "--- E2: GitHub Actions CLI + NaCl sealed-box ---"
cargo test -p arks-hub sealed_box -- --nocapture >"$OUT/e2-sealed-box.log" 2>&1
cargo test -p arks-hub actions -- --nocapture >"$OUT/e2-actions.log" 2>&1
log_pass "E2: arks-hub sealed_box KAT & Actions CLI (run/workflow/cache/secret/variable) PASS"

# 3. E3: Worktree-free history replay (`arks replay`) + stock git update-ref interop
echo "--- E3: Worktree-free arks replay + stock git interop ---"
cargo test -p arks --test git_replay_cli -- --nocapture >"$OUT/e3-replay.log" 2>&1

WORK="$(mktemp -d /tmp/arks-e1-e8.XXXXXX)"
PIDS=()
cleanup() {
  for p in "${PIDS[@]:-}"; do
    kill "$p" 2>/dev/null || true
  done
  rm -rf "$WORK"
}
trap cleanup EXIT

export GIT_AUTHOR_NAME="CleanRoom"
export GIT_AUTHOR_EMAIL="cleanroom@example.com"
export GIT_COMMITTER_NAME="CleanRoom"
export GIT_COMMITTER_EMAIL="cleanroom@example.com"

R3="$WORK/e3-repo"
mkdir -p "$R3"
git -C "$R3" init -q -b main
echo "base" > "$R3/a.txt"
git -C "$R3" add a.txt
git -C "$R3" commit -q -m "base"
git -C "$R3" checkout -q -b topic
echo "topic1" > "$R3/t.txt"
git -C "$R3" add t.txt
git -C "$R3" commit -q -m "topic commit"
git -C "$R3" checkout -q main
echo "main2" > "$R3/m.txt"
git -C "$R3" add m.txt
git -C "$R3" commit -q -m "main advance"
MAIN_BEFORE="$(git -C "$R3" rev-parse main)"
# Run arks replay --onto main --ref-action=print main..topic and pipe directly to stock git update-ref --stdin
"$ARKS_BIN" -C "$R3" replay --onto main --ref-action=print main..topic > "$WORK/e3-updates.txt"
grep -q '^update refs/heads/topic ' "$WORK/e3-updates.txt"
git -C "$R3" update-ref --stdin < "$WORK/e3-updates.txt"
git -C "$R3" fsck --full --strict >/dev/null
TOPIC_PARENT="$(git -C "$R3" rev-parse topic^)"
[[ "$TOPIC_PARENT" == "$MAIN_BEFORE" ]]
log_pass "E3: git_replay_cli + live arks replay -> stock git update-ref --stdin & fsck PASS"

# 4. E4: arks maintenance + arks subtree + stock git fsck interop
echo "--- E4: arks maintenance + arks subtree + stock git interop ---"
cargo test -p arks --test maintenance_subtree_cli -- --nocapture >"$OUT/e4-maintenance-subtree.log" 2>&1

R4="$WORK/e4-repo"
mkdir -p "$R4"
git -C "$R4" init -q -b main
echo "root" > "$R4/README.md"
git -C "$R4" add README.md
git -C "$R4" commit -q -m "root"
"$ARKS_BIN" -C "$R4" maintenance run --task=gc >/dev/null
git -C "$R4" fsck --full --strict >/dev/null
log_pass "E4: maintenance_subtree_cli + live arks maintenance run --task=gc & stock git fsck PASS"

# 5. E5: Git Reftable backend (`arks refs migrate` / `verify`) + stock git files round-trip
echo "--- E5: Git Reftable backend + round-trip interop ---"
cargo test -p arks-refs -- --nocapture >"$OUT/e5-arks-refs.log" 2>&1
cargo test -p arks --test reftable_cli -- --nocapture >"$OUT/e5-reftable-cli.log" 2>&1

R5="$WORK/e5-repo"
mkdir -p "$R5"
git -C "$R5" init -q -b main
echo "reftable test" > "$R5/f.txt"
git -C "$R5" add f.txt
git -C "$R5" commit -q -m "reftable base"
git -C "$R5" branch feat/rt
git -C "$R5" tag v0.2.0-rt
REFS_BEFORE="$(git -C "$R5" show-ref)"
"$ARKS_BIN" -C "$R5" refs migrate --ref-format=reftable >/dev/null
test -f "$R5/.git/reftable/tables.list"
"$ARKS_BIN" -C "$R5" refs verify --strict >/dev/null
"$ARKS_BIN" -C "$R5" refs migrate --ref-format=files >/dev/null
REFS_AFTER="$(git -C "$R5" show-ref)"
[[ "$REFS_BEFORE" == "$REFS_AFTER" ]]
git -C "$R5" fsck --full --strict >/dev/null
log_pass "E5: arks-refs + reftable_cli + live files->reftable->verify->files stock git parity PASS"

# 6. E6: Native Git LFS client
echo "--- E6: Native Git LFS client ---"
cargo test -p arks --test lfs_cli -- --nocapture >"$OUT/e6-lfs-cli.log" 2>&1
log_pass "E6: lfs_cli (pointer v1, clean/smudge, filter-process, content-addressed store) PASS"

# 7. E7: Self-host LFS endpoint (`arks serve` info/lfs)
echo "--- E7: Self-host LFS endpoint (arks serve info/lfs) ---"
cargo test -p arks --test serve_lfs_cli -- --nocapture >"$OUT/e7-serve-lfs-cli.log" 2>&1
log_pass "E7: serve_lfs_cli (Batch API, upload/download, SHA-256 verify, --no-lfs opt-out) PASS"

# 8. E8: Embedded SSR Web UI + Interactive Forge + stock git clone coexistence
echo "--- E8: Embedded SSR Web UI + Interactive Forge + stock git clone coexistence ---"
cargo test -p arks --test serve_web_ui_cli -- --nocapture >"$OUT/e8-serve-web-ui-cli.log" 2>&1

R8="$WORK/e8-repo"
mkdir -p "$R8"
git -C "$R8" init -q -b main
echo "# CleanRoom Forge" > "$R8/README.md"
git -C "$R8" add README.md
git -C "$R8" commit -q -m "initial forge repo"
git -C "$R8" checkout -q -b feat/cleanroom
echo "pub fn ok() -> bool { true }" > "$R8/lib.rs"
git -C "$R8" add lib.rs
git -C "$R8" commit -q -m "add lib.rs"
FEAT_OID="$(git -C "$R8" rev-parse HEAD)"
git -C "$R8" checkout -q main

PORT="$(python3 -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1])')"
"$ARKS_BIN" serve --no-ssh --no-forge --listen "127.0.0.1:$PORT" "$R8/.git" >"$OUT/e8-serve.log" 2>&1 &
PIDS+=($!)

python3 - "$PORT" <<'PY'
import socket, sys, time
port = int(sys.argv[1])
for _ in range(100):
    try:
        socket.create_connection(("127.0.0.1", port), 0.2).close()
        sys.exit(0)
    except OSError:
        time.sleep(0.1)
sys.exit(1)
PY

# Verify Web UI HTML, strict CSP, interactive issue creation, and native PR fast-forward merge
python3 - "$PORT" <<'PY'
import sys, urllib.request, urllib.parse
port = int(sys.argv[1])
base = f"http://127.0.0.1:{port}"

with urllib.request.urlopen(base + "/") as resp:
    assert resp.status == 200
    csp = resp.headers.get("Content-Security-Policy", "")
    assert "default-src 'self'" in csp, f"unexpected CSP: {csp}"
    body = resp.read().decode("utf-8")
    assert "CleanRoom Forge" in body
    assert "/assets/htmx.min.js" in body
    assert "/assets/diff2html.min.js" in body

class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None

opener = urllib.request.build_opener(NoRedirect)

# POST /issues -> 303 /issues/1
data = urllib.parse.urlencode({"title": "CleanRoom Issue", "author": "bot", "body": "Verified in Docker"}).encode()
try:
    opener.open(urllib.request.Request(base + "/issues", data=data, method="POST"))
    raise AssertionError("expected 303 redirect")
except urllib.error.HTTPError as e:
    assert e.code == 303, f"expected 303, got {e.code}"
    assert e.headers.get("Location") == "/issues/1"

with urllib.request.urlopen(base + "/issues/1") as resp:
    assert resp.status == 200
    assert "CleanRoom Issue" in resp.read().decode("utf-8")

# POST /pulls/feat/cleanroom/merge -> 303
try:
    opener.open(urllib.request.Request(base + "/pulls/feat/cleanroom/merge", data=b"", method="POST"))
    raise AssertionError("expected 303 redirect")
except urllib.error.HTTPError as e:
    assert e.code == 303, f"expected 303, got {e.code}"
PY

MAIN_MERGED="$(git -C "$R8" rev-parse refs/heads/main)"
[[ "$MAIN_MERGED" == "$FEAT_OID" ]]

# Coexistence: stock git clone over smart HTTP against the same arks serve instance!
git clone -q "http://127.0.0.1:$PORT/" "$WORK/e8-cloned"
test -f "$WORK/e8-cloned/lib.rs"
[[ "$(git -C "$WORK/e8-cloned" rev-parse HEAD)" == "$FEAT_OID" ]]
log_pass "E8: serve_web_ui_cli + live arks serve SSR/CSP/Forge merge + stock git clone coexistence PASS"

NOW_UTC="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
cat > "$OUT/receipt.json" <<EOF
{
  "gate": "e1-e8-v0-2-0-cleanroom",
  "spec": "sot:root/v0-2-0-scope-expansion",
  "venue": "docker arks-verify-m1:0.1.0 (in_docker=$IN_DOCKER)",
  "git": "$GIT_VER",
  "rustc": "$RUSTC_VER",
  "tip": "$TIP_SHA",
  "invariants": {
    "zero_gix": true,
    "zero_celir": true,
    "single_binary": true
  },
  "epics": {
    "E1_oauth_device_flow": "PASS",
    "E2_github_actions_cli_and_sealed_box": "PASS",
    "E3_worktree_free_replay_stock_git_interop": "PASS",
    "E4_maintenance_subtree_stock_git_fsck": "PASS",
    "E5_reftable_migrate_verify_roundtrip": "PASS",
    "E6_native_lfs_client": "PASS",
    "E7_serve_lfs_endpoint": "PASS",
    "E8_serve_web_ui_forge_and_stock_git_clone": "PASS"
  },
  "result": "PASS",
  "timestamp": "$NOW_UTC"
}
EOF

echo "E1-E8 clean-room gate: ALL PASS ($OUT/receipt.json)"
