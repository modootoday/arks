#!/usr/bin/env bash
# Claimed residual gate B3 — hub HTTPS forge remote e2e.
# SoT: .spec/sot/arks-claimed-residual-gates.sot.md
# Product forge in-process TLS + arks hub client (ureq + ARKS_HUB_CA_FILE); no gh.
# MUST NOT invent PASS from hub --local-tls smoke alone.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

SOT="${ARKS_CLAIMED_RESIDUAL_SOT:-.spec/sot/arks-claimed-residual-gates.sot.md}"
TIP="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
echo "claimed-hub-https-e2e: tip=${TIP}"

if [[ ! -f "$SOT" ]]; then
  echo "claimed-hub-https-e2e: missing SoT $SOT" >&2
  exit 1
fi

command -v openssl >/dev/null || {
  echo "claimed-hub-https-e2e: openssl missing (oracle for PEM mint)" >&2
  exit 1
}
command -v curl >/dev/null || {
  echo "claimed-hub-https-e2e: curl missing (stock HTTPS oracle)" >&2
  exit 1
}
command -v python3 >/dev/null || {
  echo "claimed-hub-https-e2e: python3 missing" >&2
  exit 1
}

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
cargo build -p arks --release
ARKS_BIN="${ARKS_BIN:-${CARGO_TARGET_DIR:-target}/release/arks}"
[[ -x "$ARKS_BIN" ]] || {
  echo "claimed-hub-https-e2e: arks binary missing: $ARKS_BIN" >&2
  exit 1
}

WORK="$(mktemp -d /tmp/arks-b3.XXXXXX)"
SERVER_PID=""
cleanup() {
  if [[ -n "$SERVER_PID" ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
  rm -rf "$WORK"
}
trap cleanup EXIT

openssl req -x509 -newkey rsa:2048 -nodes \
  -keyout "$WORK/ca.key" -out "$WORK/ca.crt" -days 1 \
  -subj "/CN=arks-b3-ca" >/dev/null 2>&1
openssl req -newkey rsa:2048 -nodes \
  -keyout "$WORK/server.key" -out "$WORK/server.csr" \
  -subj "/CN=localhost" >/dev/null 2>&1
printf 'subjectAltName=DNS:localhost,IP:127.0.0.1\n' >"$WORK/ext.cnf"
openssl x509 -req -in "$WORK/server.csr" -CA "$WORK/ca.crt" -CAkey "$WORK/ca.key" \
  -CAcreateserial -out "$WORK/server.crt" -days 1 -extfile "$WORK/ext.cnf" >/dev/null 2>&1

HUB_DATA="$(mktemp -d /tmp/arks-b3-hub.XXXXXX)"
FORGE_PORT="$(python3 - <<'PY'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
)"
FORGE_LISTEN="127.0.0.1:${FORGE_PORT}"

"$ARKS_BIN" serve --no-ssh --no-http \
  --forge-listen "$FORGE_LISTEN" \
  --forge-data "$HUB_DATA" \
  --tls-cert "$WORK/server.crt" \
  --tls-key "$WORK/server.key" \
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
raise SystemExit("B3 forge TLS listener not ready")
PY

HTTP_CODE="$(curl -fsS --cacert "$WORK/ca.crt" \
  -o "$WORK/curl.health.body" -w '%{http_code}' \
  "https://${FORGE_LISTEN}/health")"
[[ "$HTTP_CODE" == "200" ]] || {
  echo "claimed-hub-https-e2e: curl HTTPS /health failed http=${HTTP_CODE}" >&2
  cat "$WORK/server.err" >&2 || true
  exit 1
}
grep -F '"service":"arks-hub"' "$WORK/curl.health.body" >/dev/null
grep -E "listening on https://" "$WORK/server.err" >/dev/null
grep -F "forge API: in-process TLS terminate armed" "$WORK/server.err" >/dev/null

export ARKS_HUB_API="https://${FORGE_LISTEN}"
export ARKS_HUB_CA_FILE="$WORK/ca.crt"
export ARKS_HUB_TOKEN="local-dev-token"
SMOKE_OUT="$("$ARKS_BIN" hub smoke --path /user 2>&1)" || {
  echo "$SMOKE_OUT" >&2
  cat "$WORK/server.err" >&2 || true
  exit 1
}
echo "$SMOKE_OUT" | grep -F "https_smoke=ok" >/dev/null
echo "$SMOKE_OUT" | grep -F "spawns_gh=never" >/dev/null

"$ARKS_BIN" hub auth login >/dev/null
REPO_OUT="$("$ARKS_BIN" hub repo list 2>&1)" || {
  echo "$REPO_OUT" >&2
  exit 1
}

echo "claimed-hub-https-e2e: B3 product forge HTTPS + hub client journey green"

if [[ -f /.dockerenv ]]; then
  TS="$(date -u +%Y%m%d%H%M%S)"
  OUT=".spec/evidence/${TS}-claimed-hub-https"
  mkdir -p "$OUT"
  cp "$WORK/server.err" "$OUT/server.err"
  cp "$WORK/curl.health.body" "$OUT/curl-health.body"
  printf '%s\n' "$SMOKE_OUT" >"$OUT/hub-smoke.out"
  printf '%s\n' "$REPO_OUT" >"$OUT/hub-repo-list.out"
  python3 - "$OUT/receipt.json" "$TIP" "${ARKS_JOURNEY_IMAGE_TAG:-arks-verify-m1:0.1.0}" <<'PY'
import json, sys, time
out, tip, image = sys.argv[1:]
receipt = {
    "schema": "arks-claimed-residual/v1",
    "gate": "B3",
    "disposition": "verified-product-path",
    "venue": "docker-forge-https-hub-client",
    "tip": tip,
    "image_tag": image,
    "cipher_path": "rustls-server + operator PEM (forge module)",
    "client": "arks hub (ureq + ARKS_HUB_CA_FILE); curl --cacert oracle",
    "checks": [
        "arks serve --tls-cert/--tls-key forge https banner",
        "stock curl HTTPS /health 200",
        "arks hub smoke GET /user over https",
        "arks hub repo list authenticated",
    ],
    "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
    "note": "hub --local-tls smoke alone does not satisfy B3; this receipt is product forge terminate.",
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
PY
  echo "claimed-hub-https-e2e: receipt=$OUT/receipt.json"
else
  echo "claimed-hub-https-e2e: host run only; no .spec/evidence receipt written"
fi
