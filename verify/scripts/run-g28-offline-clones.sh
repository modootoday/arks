#!/usr/bin/env bash
# G28 offline clone closure — J-CLONE-LOCAL-01 + J-CLONE-PATH-01 with arks.
# Aligns with G16 seeds/oracles (smart-HTTP git-http-backend peer; path + file://).
# SoT: g28-live-private-arks-journeys; catalog: clean-room-journey-compare.
# Honesty: functional offline only — not G16 timing; not live forge; not G26/G27 scale.
set -euo pipefail

ROOT="${ARKS_G28_ROOT:-/workspace}"
if [[ -d /workspaces/gir/verify ]]; then
  ROOT="${ARKS_G28_ROOT:-/workspaces/gir}"
fi
cd "$ROOT"

mkdir -p verify/g28/out
WORK="${ARKS_G28_OFFLINE_WORK:-$ROOT/verify/g28/out/offline-work}"
RECEIPT="${ARKS_G28_OFFLINE_RECEIPT_OUT:-$ROOT/verify/g28/out/offline-clones-receipt.json}"
RESULTS_JSON="${ARKS_G28_OFFLINE_RESULTS_OUT:-$ROOT/verify/g28/out/offline-clones-journeys.json}"
REVISION="$(git -C "$ROOT" rev-parse HEAD 2>/dev/null || echo unknown)"
STOCK_GIT="$(git --version)"
IMAGE_TAG="${ARKS_JOURNEY_IMAGE_TAG:-arks-verify-m1:0.1.0}"
# Functional seed: same shape as G16 (README + bulk + second commit). Light bulk vs G19 timing.
BULK_FILES="${ARKS_G28_OFFLINE_BULK_FILES:-16}"

rm -rf "$WORK"
mkdir -p "$WORK"
echo '[]' >"$RESULTS_JSON"

record() {
  local id="$1" status="$2" detail="$3"
  python3 - "$RESULTS_JSON" "$id" "$status" "$detail" <<'PY'
import json, sys
path, id_, status, detail = sys.argv[1:5]
with open(path, encoding="utf-8") as f:
    rows = json.load(f)
rows.append({"id": id_, "status": status, "detail": detail})
with open(path, "w", encoding="utf-8") as f:
    json.dump(rows, f, indent=2)
    f.write("\n")
print(f"G28-offline: {id_} => {status} ({detail})")
PY
}

write_receipt() {
  local disposition="$1"
  local detail="$2"
  python3 - "$RECEIPT" "$RESULTS_JSON" "$disposition" "$detail" "$REVISION" "$STOCK_GIT" "$IMAGE_TAG" <<'PY'
import json, sys
out, results_path, disposition, detail, revision, stock_git, image_tag = sys.argv[1:8]
with open(results_path, encoding="utf-8") as f:
    journeys = json.load(f)
fail = sum(1 for j in journeys if j.get("status") == "FAIL")
passt = sum(1 for j in journeys if j.get("status") == "PASS")
receipt = {
    "schema": "arks-g28-offline-clone-journeys/v1",
    "gate": "G28",
    "name": "g28-offline-clone-journeys",
    "disposition": disposition,
    "detail": detail,
    "venue": "clean-room",
    "network": "offline",
    "revision": revision,
    "stock_git": stock_git,
    "image_tag": image_tag,
    "seed_align": "G16 J-CLONE-LOCAL-01 / J-CLONE-PATH-01 (git http-backend peer; path + file://; README+bulk bare)",
    "counts": {"PASS": passt, "FAIL": fail, "total": len(journeys)},
    "journeys": journeys,
    "honesty": "offline functional clone closure for G28 N/A rows; != G16 timing; != live forge; != G26/G27 scale",
    "sot": "sot:root/g28-live-private-arks-journeys",
    "catalog": "sot:root/clean-room-journey-compare",
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
print(f"G28-offline: receipt written to {out} disposition={disposition} PASS={passt} FAIL={fail}")
PY
}

start_http_peer() {
  local project_root="$1" log_path="$2" url_file="$3"
  python3 - "$project_root" "$log_path" "$url_file" <<'PY' &
import os, socket, subprocess, sys, threading, time
from http.server import BaseHTTPRequestHandler, HTTPServer

project_root, log_path, url_file = sys.argv[1:4]

class Handler(BaseHTTPRequestHandler):
    def log_message(self, fmt, *args):
        with open(log_path, "a", encoding="utf-8") as f:
            f.write("%s - %s\n" % (self.address_string(), fmt % args))

    def _handle(self):
        length = int(self.headers.get("Content-Length") or 0)
        body = self.rfile.read(length) if length else b""
        path = self.path
        query = ""
        if "?" in path:
            path, query = path.split("?", 1)
        env = os.environ.copy()
        env.update({
            "GIT_PROJECT_ROOT": project_root,
            "GIT_HTTP_EXPORT_ALL": "1",
            "REQUEST_METHOD": self.command,
            "PATH_INFO": path,
            "QUERY_STRING": query,
            "CONTENT_TYPE": self.headers.get("Content-Type") or "application/x-git-upload-pack-request",
            "CONTENT_LENGTH": str(len(body)),
        })
        proc = subprocess.run(
            ["git", "http-backend"],
            input=body,
            capture_output=True,
            env=env,
        )
        if proc.returncode != 0:
            self.send_error(500, "git http-backend failed")
            with open(log_path, "a", encoding="utf-8") as f:
                f.write(proc.stderr.decode("utf-8", "replace") + "\n")
            return
        cgi = proc.stdout
        sep = b"\r\n\r\n" if b"\r\n\r\n" in cgi else b"\n\n"
        head, rest = cgi.split(sep, 1) if sep in cgi else (b"", cgi)
        status = 200
        headers = []
        for line in head.decode("latin1", "replace").splitlines():
            if line.lower().startswith("status:"):
                status = int(line.split(":", 1)[1].strip().split()[0])
            elif ":" in line:
                k, v = line.split(":", 1)
                headers.append((k.strip(), v.strip()))
        self.send_response(status)
        self.send_header("Content-Length", str(len(rest)))
        for k, v in headers:
            if k.lower() == "status":
                continue
            self.send_header(k, v)
        self.end_headers()
        self.wfile.write(rest)

    def do_GET(self):
        self._handle()

    def do_POST(self):
        self._handle()

sock = socket.socket()
sock.bind(("127.0.0.1", 0))
port = sock.getsockname()[1]
sock.close()
httpd = HTTPServer(("127.0.0.1", port), Handler)
with open(url_file, "w", encoding="utf-8") as f:
    f.write(f"http://127.0.0.1:{port}\n")
threading.Thread(target=httpd.serve_forever, daemon=True).start()
while True:
    time.sleep(3600)
PY
  PEER_PID=$!
}

if ! command -v git >/dev/null; then
  echo "G28-offline: stock git missing in image" >&2
  exit 1
fi
echo "G28-offline: stock git ${STOCK_GIT}"

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/cargo-target}"
echo "G28-offline: build arks (release)"
cargo build -p arks --release
ARKS_BIN="$(find "${CARGO_TARGET_DIR}/release" target/release -name arks -type f 2>/dev/null | head -n1 || true)"
if [[ -z "${ARKS_BIN}" || ! -x "${ARKS_BIN}" ]]; then
  echo "G28-offline: arks binary missing after build" >&2
  exit 1
fi
echo "G28-offline: ARKS_BIN=${ARKS_BIN}"

seed_bare() {
  local parent="$1" name="$2" extra="$3"
  local work="${parent}/${name}-work"
  mkdir -p "$work"
  git -C "$work" init -b main >/dev/null
  git -C "$work" config user.email "arks@example.com"
  git -C "$work" config user.name "arks"
  printf 'hello arks g16\n' >"$work/README"
  if [[ "$extra" -gt 0 ]]; then
    mkdir -p "$work/bulk"
    local i
    for ((i = 0; i < extra; i++)); do
      python3 - "$work/bulk" "$i" <<'PY'
import pathlib, sys
bulk, i = pathlib.Path(sys.argv[1]), int(sys.argv[2])
body = (f"arks g16 bulk {i}\n" * 32)
(bulk / f"f{i:03d}.txt").write_text(body, encoding="utf-8")
PY
    done
  fi
  git -C "$work" add -A
  git -C "$work" -c commit.gpgsign=false commit -m "init" >/dev/null
  if [[ "$extra" -gt 0 ]]; then
    printf 'hello arks g16\nwave-2\n' >"$work/README"
    git -C "$work" add README
    git -C "$work" -c commit.gpgsign=false commit -m "touch" >/dev/null
  fi
  git -C "$parent" clone --bare "$work" "$name" >/dev/null
  echo "${parent}/${name}"
}

PEER_PID=""
cleanup_peer() {
  if [[ -n "${PEER_PID}" ]] && kill -0 "$PEER_PID" 2>/dev/null; then
    kill "$PEER_PID" 2>/dev/null || true
    wait "$PEER_PID" 2>/dev/null || true
  fi
}
trap cleanup_peer EXIT

# --- J-CLONE-LOCAL-01: smart-HTTP local peer (git http-backend) → arks clone ---
PEER_ROOT="$WORK/j-clone-peer"
mkdir -p "$PEER_ROOT"
seed_bare "$PEER_ROOT" "app.git" "$BULK_FILES" >/dev/null
PEER_LOG="$WORK/http-peer.log"
PEER_URL_FILE="$WORK/peer-url.txt"
: >"$PEER_LOG"
start_http_peer "$PEER_ROOT" "$PEER_LOG" "$PEER_URL_FILE"

for _ in $(seq 1 100); do
  if [[ -f "$PEER_URL_FILE" ]]; then
    break
  fi
  sleep 0.05
done
if [[ ! -f "$PEER_URL_FILE" ]]; then
  echo "G28-offline: smart-HTTP peer failed to start" >&2
  record "J-CLONE-LOCAL-01" "FAIL" "peer start"
  record "J-CLONE-PATH-01" "FAIL" "skipped after peer fail"
  write_receipt "fail" "smart-HTTP peer start failed"
  exit 1
fi
PEER_BASE="$(tr -d '\r\n' <"$PEER_URL_FILE")"
CLONE_URL="${PEER_BASE}/app.git"
echo "G28-offline: peer ${CLONE_URL}"

DEST_HTTP="$WORK/j-clone/clone-arks"
mkdir -p "$WORK/j-clone"
if "${ARKS_BIN}" clone "$CLONE_URL" "$DEST_HTTP" >/dev/null 2>"$WORK/j-clone-local.err"; then
  if [[ -e "$DEST_HTTP/.git" || -e "$DEST_HTTP/HEAD" ]]; then
    record "J-CLONE-LOCAL-01" "PASS" "arks clone smart-HTTP local peer (G16-aligned seed)"
  else
    record "J-CLONE-LOCAL-01" "FAIL" "clone exit 0 but missing .git/HEAD"
  fi
else
  detail="$(tr '\n' ' ' <"$WORK/j-clone-local.err" | head -c 240)"
  record "J-CLONE-LOCAL-01" "FAIL" "arks clone failed: ${detail}"
fi
cleanup_peer
PEER_PID=""

# --- J-CLONE-PATH-01: path + file:// clone ---
PATH_ROOT="$WORK/j-clone-path-src"
mkdir -p "$PATH_ROOT"
BARE_PATH="$(seed_bare "$PATH_ROOT" "path-app.git" "$BULK_FILES")"
ABS_BARE="$(python3 - "$BARE_PATH" <<'PY'
import pathlib, sys
print(pathlib.Path(sys.argv[1]).resolve().as_posix())
PY
)"
if [[ "$ABS_BARE" == /* ]]; then
  FILE_URL="file://${ABS_BARE}"
else
  FILE_URL="file:///${ABS_BARE}"
fi

DEST_PATH="$WORK/j-clone-path/clone-path-arks"
DEST_FILE="$WORK/j-clone-path/clone-file-arks"
mkdir -p "$WORK/j-clone-path"
PATH_OK=0
FILE_OK=0
if "${ARKS_BIN}" clone "$BARE_PATH" "$DEST_PATH" >/dev/null 2>"$WORK/j-clone-path.err"; then
  if [[ -e "$DEST_PATH/.git" || -e "$DEST_PATH/HEAD" ]] && [[ -f "$DEST_PATH/README" || -f "$DEST_PATH/README.md" ]]; then
    PATH_OK=1
  fi
fi
if "${ARKS_BIN}" clone "$FILE_URL" "$DEST_FILE" >/dev/null 2>"$WORK/j-clone-file.err"; then
  if [[ -e "$DEST_FILE/.git" || -e "$DEST_FILE/HEAD" ]]; then
    FILE_OK=1
  fi
fi
if [[ "$PATH_OK" -eq 1 && "$FILE_OK" -eq 1 ]]; then
  record "J-CLONE-PATH-01" "PASS" "arks path + file:// clone (G16-aligned seed)"
else
  record "J-CLONE-PATH-01" "FAIL" "path_ok=${PATH_OK} file_ok=${FILE_OK}"
fi

FAIL_COUNT="$(python3 - "$RESULTS_JSON" <<'PY'
import json, sys
with open(sys.argv[1], encoding="utf-8") as f:
    rows = json.load(f)
print(sum(1 for j in rows if j.get("status") == "FAIL"))
PY
)"

if [[ "$FAIL_COUNT" -gt 0 ]]; then
  write_receipt "fail" "offline clone journey FAIL"
  echo "G28-offline: FAIL (${FAIL_COUNT})"
  exit 1
fi

write_receipt "closed-implement" "J-CLONE-LOCAL-01 + J-CLONE-PATH-01 PASS offline with arks"
echo "G28-offline: PASS"
