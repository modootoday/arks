#!/usr/bin/env bash
# Claimed residual gate B9 — product in-process TLS terminate.
# SoT: .spec/sot/arks-claimed-residual-gates.sot.md
# Operator-supplied PEMs on `arks serve --no-ssh`; stock curl over HTTPS.
# MUST NOT invent PASS from hub --local-tls smoke or ops reverse-proxy alone.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

SOT="${ARKS_CLAIMED_RESIDUAL_SOT:-.spec/sot/arks-claimed-residual-gates.sot.md}"
TIP="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
echo "claimed-tls-e2e: tip=${TIP}"

if [[ ! -f "$SOT" ]]; then
  echo "claimed-tls-e2e: missing SoT $SOT" >&2
  exit 1
fi

command -v openssl >/dev/null || {
  echo "claimed-tls-e2e: openssl missing (oracle for PEM mint; product never spawns openssl)" >&2
  exit 1
}
command -v curl >/dev/null || {
  echo "claimed-tls-e2e: curl missing (stock HTTPS client oracle)" >&2
  exit 1
}
command -v python3 >/dev/null || {
  echo "claimed-tls-e2e: python3 missing" >&2
  exit 1
}

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
cargo build -p arks --release
ARKS_BIN="${ARKS_BIN:-${CARGO_TARGET_DIR:-target}/release/arks}"
[[ -x "$ARKS_BIN" ]] || {
  echo "claimed-tls-e2e: arks binary missing: $ARKS_BIN" >&2
  exit 1
}

WORK="$(mktemp -d /tmp/arks-b9.XXXXXX)"
SERVER_PID=""
cleanup() {
  if [[ -n "$SERVER_PID" ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
  rm -rf "$WORK"
}
trap cleanup EXIT

# Operator-supplied CA + server cert (CN/SAN localhost) — product loads PEMs only.
openssl req -x509 -newkey rsa:2048 -nodes \
  -keyout "$WORK/ca.key" -out "$WORK/ca.crt" -days 1 \
  -subj "/CN=arks-b9-ca" >/dev/null 2>&1
openssl req -newkey rsa:2048 -nodes \
  -keyout "$WORK/server.key" -out "$WORK/server.csr" \
  -subj "/CN=localhost" >/dev/null 2>&1
printf 'subjectAltName=DNS:localhost,IP:127.0.0.1\n' >"$WORK/ext.cnf"
openssl x509 -req -in "$WORK/server.csr" -CA "$WORK/ca.crt" -CAkey "$WORK/ca.key" \
  -CAcreateserial -out "$WORK/server.crt" -days 1 -extfile "$WORK/ext.cnf" >/dev/null 2>&1

SEED="$(mktemp -d /tmp/arks-b9-seed.XXXXXX)"
git -C "$SEED" init -q
git -C "$SEED" config user.name b9
git -C "$SEED" config user.email b9@example.invalid
printf 'b9 tls\n' >"$SEED/README"
git -C "$SEED" add README
git -C "$SEED" commit -q -m b9
git clone --bare -q "$SEED" "$WORK/repo.git"
rm -rf "$SEED"

PORT="$(python3 - <<'PY'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
)"
LISTEN="127.0.0.1:${PORT}"

"$ARKS_BIN" serve --no-ssh --no-forge \
  --listen "$LISTEN" \
  --tls-cert "$WORK/server.crt" \
  --tls-key "$WORK/server.key" \
  "$WORK/repo.git" >"$WORK/server.out" 2>"$WORK/server.err" &
SERVER_PID=$!

python3 - "$PORT" <<'PY'
import socket, sys, time
port = int(sys.argv[1])
for _ in range(200):
    try:
        with socket.create_connection(("127.0.0.1", port), timeout=.2):
            raise SystemExit(0)
    except OSError:
        time.sleep(.05)
raise SystemExit("B9 TLS listener not ready")
PY

# Stock curl HTTPS against product in-process TLS (cacert = operator CA).
HTTP_CODE="$(curl -fsS --cacert "$WORK/ca.crt" \
  -o "$WORK/curl.body" -w '%{http_code}' \
  "https://127.0.0.1:${PORT}/info/refs?service=git-upload-pack")"
[[ "$HTTP_CODE" == "200" ]] || {
  echo "claimed-tls-e2e: curl HTTPS failed http=${HTTP_CODE}" >&2
  cat "$WORK/server.err" >&2 || true
  exit 1
}
grep -F "service=git-upload-pack" "$WORK/curl.body" >/dev/null
grep -E "listening on https://" "$WORK/server.err" >/dev/null
grep -F "in-process TLS terminate armed" "$WORK/server.err" >/dev/null

echo "claimed-tls-e2e: B9 product HTTPS terminate + stock curl green"

if [[ -f /.dockerenv ]]; then
  TS="$(date -u +%Y%m%d%H%M%S)"
  OUT=".spec/evidence/${TS}-claimed-tls"
  mkdir -p "$OUT"
  cp "$WORK/server.err" "$OUT/server.err"
  cp "$WORK/curl.body" "$OUT/curl-info-refs.body"
  python3 - "$OUT/receipt.json" "$TIP" "${ARKS_JOURNEY_IMAGE_TAG:-arks-verify-m1:0.1.0}" <<'PY'
import json, sys, time
out, tip, image = sys.argv[1:]
receipt = {
    "schema": "arks-claimed-residual/v1",
    "gate": "B9",
    "disposition": "verified-product-path",
    "venue": "docker-in-process-tls",
    "tip": tip,
    "image_tag": image,
    "cipher_path": "rustls-server + operator PEM",
    "client": "curl --cacert",
    "checks": [
        "arks serve --tls-cert/--tls-key banner https",
        "stock curl HTTPS info/refs 200",
    ],
    "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
    "note": "Ops reverse-proxy alone does not satisfy B9; this receipt is product terminate.",
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
PY
  echo "claimed-tls-e2e: receipt=$OUT/receipt.json"
else
  echo "claimed-tls-e2e: host run only; no .spec/evidence receipt written"
fi
