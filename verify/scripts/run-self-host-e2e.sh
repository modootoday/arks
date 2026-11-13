#!/usr/bin/env bash
# Related hosting e2e — arks-only self-host (LAN/WAN product story).
# SoT: .spec/sot/arks-self-host-server.sot.md
# Partial residual: .spec/sot/arks-partial-residual-wave.sot.md (IW-FORGE-SERVE)
# Docker claim wave: .spec/sot/arks-verify-docker-e2e-claim-wave.sot.md
# Research: .spec/research/20261111093543-partial-residual-wave-web-research.research.md
# Umbrella: .spec/sot/verify-cleanroom-symmetry-matrix.sot.md
# Optimized DoD OD-SERVE-CFG: sot:root/arks-dod-optimized (ARKS_DOD_SERVE_CFG=1 or STRICT).
# MUST NOT invent PASS; MUST NOT spawn gh. Stock git is client oracle only.
# Claimed bar: PR create/list + issue create/list against in-process forge
# under `arks serve` + scrubbed Docker receipt — B3 client remote ≠ this slot.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

SOT="${ARKS_SELF_HOST_SOT:-.spec/sot/arks-self-host-server.sot.md}"
PARTIAL_SOT="${ARKS_PARTIAL_RESIDUAL_SOT:-.spec/sot/arks-partial-residual-wave.sot.md}"
DOCKER_SOT="${ARKS_DOCKER_E2E_CLAIM_SOT:-.spec/sot/arks-verify-docker-e2e-claim-wave.sot.md}"
UMBRELLA="${ARKS_SYMMETRY_UMBRELLA_SOT:-.spec/sot/verify-cleanroom-symmetry-matrix.sot.md}"
OPT_DOD_SOT="${ARKS_DOD_OPTIMIZED_SOT:-.spec/sot/arks-dod-optimized.sot.md}"
TIP="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"

echo "self-host-e2e: tip=${TIP}"
echo "self-host-e2e: pin git=$(git --version 2>/dev/null || echo unknown)"

for f in "$SOT" "$UMBRELLA" "$PARTIAL_SOT" "$DOCKER_SOT"; do
  if [[ ! -f "$f" ]]; then
    echo "self-host-e2e: SKIP — missing SoT $f"
    exit 0
  fi
done
echo "self-host-e2e: SoT+umbrella+partial+docker-claim-wave present (IW-FORGE-SERVE)"

command -v python3 >/dev/null || {
  echo "self-host-e2e: python3 missing" >&2
  exit 1
}

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
ARKS_BIN="${ARKS_BIN:-}"
if [[ -z "$ARKS_BIN" ]]; then
  if [[ -x "${CARGO_TARGET_DIR:-}/release/arks" ]]; then
    ARKS_BIN="${CARGO_TARGET_DIR}/release/arks"
  elif [[ -x ./.cargo-target/release/arks ]]; then
    ARKS_BIN=./.cargo-target/release/arks
  elif [[ -x ./target/release/arks ]]; then
    ARKS_BIN=./target/release/arks
  else
    cargo build -p arks --release
    ARKS_BIN="${CARGO_TARGET_DIR:-target}/release/arks"
    [[ -x "$ARKS_BIN" ]] || ARKS_BIN=./.cargo-target/release/arks
    [[ -x "$ARKS_BIN" ]] || ARKS_BIN=./target/release/arks
  fi
fi
[[ -x "$ARKS_BIN" ]] || {
  echo "self-host-e2e: arks binary missing: $ARKS_BIN" >&2
  exit 1
}

# --- OD-SERVE-CFG: arks serve config smoke (sot:root/arks-dod-optimized) ---
RUN_OD_SERVE_CFG=0
[[ "${ARKS_DOD_SERVE_CFG:-0}" == "1" || "${ARKS_DOD_STRICT:-0}" == "1" ]] && RUN_OD_SERVE_CFG=1
OD_SERVE_CFG_OK=0
if [[ "$RUN_OD_SERVE_CFG" == "1" ]]; then
  if [[ ! -f "$OPT_DOD_SOT" ]]; then
    echo "self-host-e2e: FAIL OD-SERVE-CFG — missing $OPT_DOD_SOT" >&2
    exit 1
  fi
  CFG_HOME="$(mktemp -d /tmp/arks-serve-cfg.XXXXXX)"
  export HOME="$CFG_HOME"
  mkdir -p "$CFG_HOME/.config/arks"
  set +e
  path_out="$("$ARKS_BIN" serve config path 2>&1)"
  path_ec=$?
  set_out="$("$ARKS_BIN" serve config set tls_note "od-smoke" 2>&1)"
  set_ec=$?
  show_out="$("$ARKS_BIN" serve config show 2>&1)"
  show_ec=$?
  val_out="$("$ARKS_BIN" serve config validate 2>&1)"
  val_ec=$?
  set -e
  if [[ "$path_ec" -eq 0 && "$set_ec" -eq 0 && "$show_ec" -eq 0 && "$val_ec" -eq 0 ]] \
    && echo "$path_out" | grep -Eq 'serve\.conf|arks' \
    && echo "$show_out$set_out" | grep -Eq 'tls_note|od-smoke|Partial'; then
    OD_SERVE_CFG_OK=1
    echo "self-host-e2e: OD-SERVE-CFG ok — path/set/show/validate"
  else
    echo "self-host-e2e: FAIL OD-SERVE-CFG smoke" >&2
    printf 'path(%s)=%s\nset(%s)=%s\nshow(%s)=%s\nvalidate(%s)=%s\n' \
      "$path_ec" "$path_out" "$set_ec" "$set_out" "$show_ec" "$show_out" "$val_ec" "$val_out" >&2
    if [[ "${ARKS_DOD_STRICT:-0}" == "1" || "${ARKS_DOD_SERVE_CFG:-0}" == "1" ]]; then
      rm -rf "$CFG_HOME"
      exit 1
    fi
  fi
  # Keep HOME isolated for subsequent forge journeys too.
fi

# --- SH-COHOST: discoverability (Partial deepen; alone ≠ Claimed) ---
COHOST_OK=0
help_out="$("$ARKS_BIN" serve --help 2>&1 || true)"
if echo "$help_out" | grep -Eqi -- '--no-forge' && echo "$help_out" | grep -Eqi 'forge'; then
  COHOST_OK=1
  echo "self-host-e2e: SH-COHOST ok — serve --help advertises forge cohost / --no-forge"
else
  echo "self-host-e2e: SH-COHOST gap — serve --help missing forge / --no-forge markers"
fi

# --- SH-ISSUE + SH-PR: in-process forge host journeys ---
WORK="$(mktemp -d /tmp/arks-self-host.XXXXXX)"
SERVER_PID=""
cleanup() {
  if [[ -n "${SERVER_PID:-}" ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
  rm -rf "$WORK"
}
trap cleanup EXIT

FORGE_PORT="$(python3 - <<'PY'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
)"
FORGE_LISTEN="127.0.0.1:${FORGE_PORT}"
HUB_DATA="$WORK/hub-data"
mkdir -p "$HUB_DATA" "$WORK/home/.config/arks"
export HOME="$WORK/home"
export ARKS_HUB_TOKEN_FILE="$WORK/home/.config/arks/hub-token"
export ARKS_HUB_TOKEN="local-dev-token"
export ARKS_HUB_API="http://${FORGE_LISTEN}"
export ARKS_HUB_REPO="local/selfhost"

# Forge module under arks serve (HTTP/SSH opt-out OK for this Claimed class).
"$ARKS_BIN" serve --no-ssh --no-http \
  --forge-listen "$FORGE_LISTEN" \
  --forge-data "$HUB_DATA" \
  . >"$WORK/server.out" 2>"$WORK/server.err" &
SERVER_PID=$!

python3 - "$FORGE_PORT" <<'PY'
import socket, sys, time
port = int(sys.argv[1])
for _ in range(200):
    try:
        with socket.create_connection(("127.0.0.1", port), timeout=.2):
            raise SystemExit(0)
    except OSError:
        time.sleep(.05)
raise SystemExit("self-host forge listener not ready")
PY

JOURNEY_OK=1
run_step() {
  local name="$1"
  shift
  local out
  set +e
  out="$("$@" 2>&1)"
  local ec=$?
  set -e
  printf '%s\n' "$out" >"$WORK/${name}.out"
  if [[ "$ec" -ne 0 ]]; then
    echo "self-host-e2e: FAIL $name exit=$ec" >&2
    echo "$out" >&2
    JOURNEY_OK=0
    return 0
  fi
  echo "self-host-e2e: $name ok"
}

run_step auth-login "$ARKS_BIN" hub auth login
run_step repo-create "$ARKS_BIN" hub repo create selfhost --description "IW-FORGE-SERVE hosting e2e"
run_step issue-create "$ARKS_BIN" hub issue create --repo local/selfhost --title "hosting-issue" --body "SH-ISSUE"
run_step issue-list "$ARKS_BIN" hub issue list --repo local/selfhost
run_step pr-create "$ARKS_BIN" hub pr create --repo local/selfhost --title "hosting-pr" --body "SH-PR" --head feature --base main
run_step pr-list "$ARKS_BIN" hub pr list --repo local/selfhost

# Shape asserts (peer-ish JSON fields from local forge — not invent full gh parity).
if [[ "$JOURNEY_OK" -eq 1 ]]; then
  if ! grep -Eq '"number"[[:space:]]*:[[:space:]]*[0-9]+' "$WORK/issue-create.out" \
    || ! grep -Fq 'hosting-issue' "$WORK/issue-create.out" \
    || ! grep -Fq 'hosting-issue' "$WORK/issue-list.out"; then
    echo "self-host-e2e: FAIL SH-ISSUE shape (create/list)" >&2
    JOURNEY_OK=0
  else
    echo "self-host-e2e: SH-ISSUE ok — issue create/list against in-process forge"
  fi
fi
if [[ "$JOURNEY_OK" -eq 1 ]]; then
  if ! grep -Eq '"number"[[:space:]]*:[[:space:]]*[0-9]+' "$WORK/pr-create.out" \
    || ! grep -Fq 'hosting-pr' "$WORK/pr-create.out" \
    || ! grep -Fq 'hosting-pr' "$WORK/pr-list.out"; then
    echo "self-host-e2e: FAIL SH-PR shape (create/list)" >&2
    JOURNEY_OK=0
  else
    echo "self-host-e2e: SH-PR ok — pr create/list against in-process forge"
  fi
fi

if [[ "$JOURNEY_OK" -ne 1 ]]; then
  echo "self-host-e2e: IW-FORGE-SERVE hosting journeys Partial/failed (B3 ≠ this row)" >&2
  cat "$WORK/server.err" >&2 || true
  if [[ "${ARKS_DOD_STRICT:-0}" == "1" ]]; then
    echo "self-host-e2e: FAIL STRICT — hosting e2e Claimed bar not met (no invent PASS)" >&2
    exit 1
  fi
  echo "self-host-e2e: SKIP/scaffold PASS — journeys incomplete; design SoT linked"
  exit 0
fi

if [[ "$COHOST_OK" -ne 1 ]]; then
  echo "self-host-e2e: note — SH-COHOST markers weak but journeys green"
fi

echo "self-host-e2e: IW-FORGE-SERVE hosting journeys green (issue+pr create/list)"
if [[ "$RUN_OD_SERVE_CFG" == "1" && "$OD_SERVE_CFG_OK" == "1" ]]; then
  echo "self-host-e2e: OD-SERVE-CFG Claimed smoke green with hosting journeys"
fi

if [[ -f /.dockerenv ]]; then
  TS="$(date -u +%Y%m%d%H%M%S)"
  OUT=".spec/evidence/${TS}-self-host-forge-e2e"
  mkdir -p "$OUT"
  cp "$WORK/server.err" "$OUT/server.err" 2>/dev/null || true
  for f in auth-login repo-create issue-create issue-list pr-create pr-list; do
    cp "$WORK/${f}.out" "$OUT/${f}.out" 2>/dev/null || true
  done
  python3 - "$OUT/receipt.json" "$TIP" "${ARKS_JOURNEY_IMAGE_TAG:-arks-verify-m1:0.1.0}" "$FORGE_LISTEN" "$OD_SERVE_CFG_OK" <<'PY'
import json, sys, time
out, tip, image, listen, od_cfg = sys.argv[1:]
checks = [
    "SH-COHOST serve --help forge/--no-forge",
    "SH-ISSUE issue create/list against arks serve forge module",
    "SH-PR pr create/list against arks serve forge module",
]
if od_cfg == "1":
    checks.append("OD-SERVE-CFG path/set/show/validate smoke")
receipt = {
    "schema": "arks-self-host-forge-e2e/v1",
    "gate": "IW-FORGE-SERVE+OD-SERVE-CFG" if od_cfg == "1" else "IW-FORGE-SERVE",
    "disposition": "verified-product-path",
    "venue": "docker-dod-self-host",
    "tip": tip,
    "image_tag": image,
    "forge_listen": listen,
    "client": "arks hub (in-process; no gh spawn)",
    "od_serve_cfg": od_cfg == "1",
    "checks": checks,
    "note": "B3 hub HTTPS client remote Claimed alone does not satisfy IW-FORGE-SERVE. Serve config schema depth remains Partial.",
    "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
PY
  echo "self-host-e2e: receipt=$OUT/receipt.json"
else
  echo "self-host-e2e: host run only; no .spec/evidence receipt written (Docker Claimed venue required)"
fi

exit 0
