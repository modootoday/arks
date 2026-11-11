#!/usr/bin/env bash
# DoD pillar 1 — git vs arks CLI matrix e2e (pin 2.39.5).
# Reads living CSV verify/matrices/cli-git-help-matrix.csv.
# Never invents PASS; never spawns gh from product. Stock `git` is pin oracle only.
# IW-PERSONALITY drop-in: sot:root/arks-verify-docker-e2e-claim-wave (ARKS_PERSONALITY_DROPIN=1).
# ExitCodePolicy unit greens ≠ drop-in Claimed.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

CSV="${ARKS_CLI_MATRIX_CSV:-verify/matrices/cli-git-help-matrix.csv}"
SOT="${ARKS_CLI_MATRIX_SOT:-.spec/sot/cli-git-help-matrix.sot.md}"
DOCKER_SOT="${ARKS_DOCKER_E2E_CLAIM_SOT:-.spec/sot/arks-verify-docker-e2e-claim-wave.sot.md}"
PERSONALITY_SOT="${ARKS_PERSONALITY_SOT:-.spec/sot/arks-cli-personality-factory.sot.md}"
TIP="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"

echo "cli-git-arks-matrix-e2e: tip=${TIP}"
echo "cli-git-arks-matrix-e2e: pin git=$(git --version 2>/dev/null || echo unknown)"

ARKS_BIN="${ARKS_BIN:-}"
resolve_arks_bin() {
  if [[ -n "${ARKS_BIN:-}" && -x "$ARKS_BIN" ]]; then
    return 0
  fi
  if [[ -x "${CARGO_TARGET_DIR:-}/release/arks" ]]; then
    ARKS_BIN="${CARGO_TARGET_DIR}/release/arks"
  elif [[ -x ./.cargo-target/release/arks ]]; then
    ARKS_BIN=./.cargo-target/release/arks
  elif [[ -x ./target/release/arks ]]; then
    ARKS_BIN=./target/release/arks
  elif [[ -x ./.cargo-target/debug/arks ]]; then
    ARKS_BIN=./.cargo-target/debug/arks
  elif [[ -x ./target/debug/arks ]]; then
    ARKS_BIN=./target/debug/arks
  elif [[ -n "${CARGO_TARGET_DIR:-}" && -x "${CARGO_TARGET_DIR}/debug/arks" ]]; then
    ARKS_BIN="${CARGO_TARGET_DIR}/debug/arks"
  else
    export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
    cargo build -p arks --release
    ARKS_BIN="${CARGO_TARGET_DIR:-target}/release/arks"
    [[ -x "$ARKS_BIN" ]] || ARKS_BIN=./.cargo-target/release/arks
    [[ -x "$ARKS_BIN" ]] || ARKS_BIN=./target/release/arks
  fi
}

# --- IW-PERSONALITY drop-in Claimed sample (ARKS_PERSONALITY_DROPIN=1) ---
if [[ "${ARKS_PERSONALITY_DROPIN:-0}" == "1" ]]; then
  echo "cli-git-arks-matrix-e2e: IW-PERSONALITY drop-in path (sot:root/arks-verify-docker-e2e-claim-wave)"
  for f in "$DOCKER_SOT" "$PERSONALITY_SOT"; do
    if [[ ! -f "$f" ]]; then
      echo "FAIL: missing SoT $f" >&2
      exit 1
    fi
  done
  command -v git >/dev/null || {
    echo "FAIL: pin git missing for PD-GIT compare" >&2
    exit 1
  }
  command -v python3 >/dev/null || {
    echo "FAIL: python3 missing" >&2
    exit 1
  }
  # Always rebuild so peer-shaped version/exit deepen is present (no stale binary Claimed).
  export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
  cargo build -p arks --release
  ARKS_BIN="${CARGO_TARGET_DIR:-target}/release/arks"
  [[ -x "$ARKS_BIN" ]] || ARKS_BIN=./.cargo-target/release/arks
  [[ -x "$ARKS_BIN" ]] || ARKS_BIN=./target/release/arks
  [[ -x "$ARKS_BIN" ]] || {
    echo "FAIL: arks binary missing after rebuild: $ARKS_BIN" >&2
    exit 1
  }

  WORK="$(mktemp -d /tmp/arks-personality.XXXXXX)"
  SERVER_PID=""
  cleanup_pd() {
    if [[ -n "${SERVER_PID:-}" ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
      kill "$SERVER_PID" 2>/dev/null || true
      wait "$SERVER_PID" 2>/dev/null || true
    fi
    rm -rf "$WORK"
  }
  trap cleanup_pd EXIT
  mkdir -p "$WORK"

  DROPIN_OK=1

  # PD-GIT: version stdout shape + exit 0 vs pin git --version exit 0.
  set +e
  pin_ver_out="$(git --version 2>&1)"
  pin_ver_ec=$?
  git_ver_out="$(ARKS_PERSONALITY=git "$ARKS_BIN" --version 2>&1)"
  git_ver_ec=$?
  git_bad_out="$(ARKS_PERSONALITY=git "$ARKS_BIN" --not-a-real-flag 2>&1)"
  git_bad_ec=$?
  set -e
  printf '%s\n' "$pin_ver_out" >"$WORK/pin-git-version.out"
  printf '%s\n' "$git_ver_out" >"$WORK/pd-git-version.out"
  printf '%s\n' "$git_bad_out" >"$WORK/pd-git-badflag.out"
  echo "pd-git-version-ec=$git_ver_ec" >"$WORK/pd-git-ecs.txt"
  echo "pin-git-version-ec=$pin_ver_ec" >>"$WORK/pd-git-ecs.txt"
  echo "pd-git-badflag-ec=$git_bad_ec" >>"$WORK/pd-git-ecs.txt"

  if [[ "$pin_ver_ec" -ne 0 ]] || [[ "$git_ver_ec" -ne 0 ]]; then
    echo "FAIL PD-GIT: version exit class (pin=$pin_ver_ec arks=$git_ver_ec)" >&2
    DROPIN_OK=0
  elif ! echo "$pin_ver_out" | grep -Eq '^git version '; then
    echo "FAIL PD-GIT: pin git --version unexpected shape" >&2
    DROPIN_OK=0
  elif ! echo "$git_ver_out" | grep -Eq '^git version '; then
    echo "FAIL PD-GIT: ARKS_PERSONALITY=git --version missing peer stdout shape" >&2
    DROPIN_OK=0
  elif [[ "$git_bad_ec" -eq 0 ]]; then
    echo "FAIL PD-GIT: bad-flag must be non-zero (got 0)" >&2
    DROPIN_OK=0
  else
    echo "cli-git-arks-matrix-e2e: PD-GIT ok — version shape + exit classes (badflag ec=$git_bad_ec)"
  fi

  # PD-GH: help exit 0; unknown leaf usage-class 2; pr list against local forge.
  set +e
  gh_help_out="$(ARKS_PERSONALITY=gh "$ARKS_BIN" pr --help 2>&1)"
  gh_help_ec=$?
  gh_unk_out="$(ARKS_PERSONALITY=gh "$ARKS_BIN" not-a-gh-leaf 2>&1)"
  gh_unk_ec=$?
  set -e
  printf '%s\n' "$gh_help_out" >"$WORK/pd-gh-pr-help.out"
  printf '%s\n' "$gh_unk_out" >"$WORK/pd-gh-unknown.out"
  echo "pd-gh-help-ec=$gh_help_ec" >"$WORK/pd-gh-ecs.txt"
  echo "pd-gh-unknown-ec=$gh_unk_ec" >>"$WORK/pd-gh-ecs.txt"

  if [[ "$gh_help_ec" -ne 0 ]]; then
    echo "FAIL PD-GH: pr --help exit=$gh_help_ec" >&2
    DROPIN_OK=0
  elif [[ "$gh_unk_ec" -ne 2 ]]; then
    echo "FAIL PD-GH: unknown leaf expected usage-class exit 2 (got $gh_unk_ec)" >&2
    DROPIN_OK=0
  else
    echo "cli-git-arks-matrix-e2e: PD-GH help/unknown exit classes ok"
  fi

  # Happy pr list under gh personality + local forge (host journey client, not hosting Claimed).
  FORGE_PORT="$(python3 - <<'PY'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
)"
  FORGE_LISTEN="127.0.0.1:${FORGE_PORT}"
  mkdir -p "$WORK/hub-data" "$WORK/home/.config/arks"
  export HOME="$WORK/home"
  export ARKS_HUB_TOKEN_FILE="$WORK/home/.config/arks/hub-token"
  export ARKS_HUB_TOKEN="local-dev-token"
  export ARKS_HUB_API="http://${FORGE_LISTEN}"
  export ARKS_HUB_REPO="local/pdropin"

  "$ARKS_BIN" serve --no-ssh --no-http \
    --forge-listen "$FORGE_LISTEN" \
    --forge-data "$WORK/hub-data" \
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
raise SystemExit("personality forge listener not ready")
PY

  "$ARKS_BIN" hub auth login >/dev/null
  "$ARKS_BIN" hub repo create pdropin --description "IW-PERSONALITY" >/dev/null
  set +e
  gh_list_out="$(ARKS_PERSONALITY=gh "$ARKS_BIN" pr list --repo local/pdropin 2>&1)"
  gh_list_ec=$?
  set -e
  printf '%s\n' "$gh_list_out" >"$WORK/pd-gh-pr-list.out"
  echo "pd-gh-pr-list-ec=$gh_list_ec" >>"$WORK/pd-gh-ecs.txt"
  if [[ "$gh_list_ec" -ne 0 ]]; then
    echo "FAIL PD-GH: pr list under gh personality exit=$gh_list_ec" >&2
    echo "$gh_list_out" >&2
    DROPIN_OK=0
  else
    echo "cli-git-arks-matrix-e2e: PD-GH pr list happy path ok"
  fi

  if [[ "$DROPIN_OK" -ne 1 ]]; then
    if [[ "${ARKS_DOD_STRICT:-0}" == "1" ]]; then
      echo "cli-git-arks-matrix-e2e: FAIL STRICT IW-PERSONALITY drop-in (no invent PASS)" >&2
      exit 1
    fi
    echo "cli-git-arks-matrix-e2e: IW-PERSONALITY drop-in Partial/failed"
    exit 0
  fi

  echo "cli-git-arks-matrix-e2e: IW-PERSONALITY drop-in sample greens (ExitCodePolicy units ≠ this row)"

  if [[ -f /.dockerenv ]]; then
    TS="$(date -u +%Y%m%d%H%M%S)"
    OUT=".spec/evidence/${TS}-personality-dropin"
    mkdir -p "$OUT"
    cp "$WORK"/*.out "$WORK"/*.txt "$WORK/server.err" "$OUT/" 2>/dev/null || true
    python3 - "$OUT/receipt.json" "$TIP" "${ARKS_JOURNEY_IMAGE_TAG:-arks-verify-m1:0.1.0}" <<'PY'
import json, sys, time
out, tip, image = sys.argv[1:]
receipt = {
    "schema": "arks-personality-dropin/v1",
    "gate": "IW-PERSONALITY",
    "disposition": "verified-product-path",
    "venue": "docker-dod-cli-matrix",
    "tip": tip,
    "image_tag": image,
    "checks": [
        "PD-GIT ARKS_PERSONALITY=git --version peer stdout ^git version + exit 0 vs pin",
        "PD-GIT bad-flag non-zero exit",
        "PD-GH pr --help exit 0; unknown leaf exit 2",
        "PD-GH pr list happy path against local forge",
    ],
    "note": "ExitCodePolicy OR-of-failure unit greens alone do not satisfy IW-PERSONALITY Claimed.",
    "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
PY
    echo "cli-git-arks-matrix-e2e: receipt=$OUT/receipt.json"
  else
    echo "cli-git-arks-matrix-e2e: host run only; no .spec/evidence receipt (Docker Claimed venue required)"
  fi
  exit 0
fi

resolve_arks_bin
[[ -x "$ARKS_BIN" ]] || {
  echo "FAIL: arks binary missing: $ARKS_BIN" >&2
  exit 1
}

if [[ ! -f "$CSV" ]]; then
  echo "cli-git-arks-matrix-e2e: SKIP — missing $CSV (sibling/matrix SoT not landed)"
  exit 0
fi
if [[ ! -f "$SOT" ]]; then
  echo "cli-git-arks-matrix-e2e: SKIP — missing $SOT"
  exit 0
fi

echo "cli-git-arks-matrix-e2e: csv=$CSV"

# Count Missing rows — DoD fails closed while Missing > 0 (M16 honesty).
missing=$(awk -F, 'NR>1 && $3=="Missing" {c++} END{print c+0}' "$CSV")
partial=$(awk -F, 'NR>1 && $3=="Partial" {c++} END{print c+0}' "$CSV")
refuse=$(awk -F, 'NR>1 && $3=="Refuse/N/A" {c++} END{print c+0}' "$CSV")
echo "cli-git-arks-matrix-e2e: Missing=$missing Partial=$partial Refuse/N/A=$refuse"

# Spot-check: every Partial/Parity-candidate command must not print "command not implemented".
fail=0
checked=0
while IFS=, read -r cmd category status target notes; do
  [[ "$cmd" == "git_command" ]] && continue
  case "$status" in
    Partial|Parity-candidate|Synonym-wired) ;;
    *) continue ;;
  esac
  # Skip arks-only / non-dispatch noise.
  case "$cmd" in
    ''|serve-http|lease|lock) ;;
  esac
  checked=$((checked + 1))
  # Limit sample to keep gate time bounded; full walk when Missing==0.
  if [[ "$missing" -gt 0 && "$checked" -gt 40 ]]; then
    break
  fi
  out="$("$ARKS_BIN" "$cmd" --help 2>&1 || true)"
  if echo "$out" | grep -qi 'command not implemented'; then
    echo "FAIL: $cmd still 'command not implemented' but CSV=$status"
    fail=1
  fi
done < "$CSV"

if [[ "$fail" -ne 0 ]]; then
  echo "cli-git-arks-matrix-e2e: FAIL — dispatch/CSV honesty mismatch"
  exit 1
fi

if [[ "$missing" -eq 0 ]]; then
  echo "cli-git-arks-matrix-e2e: PASS — Missing=0 and sampled dispatch honest (flag-depth still Partial)"
  exit 0
fi

# Strict DoD fails closed while Missing remain.
if [[ "${ARKS_DOD_STRICT:-0}" == "1" ]]; then
  echo "cli-git-arks-matrix-e2e: FAIL — Missing=$missing under ARKS_DOD_STRICT=1 (no invent PASS)"
  exit 1
fi

echo "cli-git-arks-matrix-e2e: smoke-only PASS (CSV readable, sample dispatch honest); Missing=$missing — full inventory DoD waits Missing=0"
echo "cli-git-arks-matrix-e2e: IW-PERSONALITY drop-in Claimed path: ARKS_PERSONALITY_DROPIN=1 (+ STRICT)"
exit 0
