#!/usr/bin/env bash
# G42 — cloud agent drop-in clean-room gate (.spec/sot/cloud-agent-dropin.sot.md §V).
#
# Simulates a hosted agent sandbox inside the clean-room image:
#   - `arks serve` HTTPS with a certificate from a private CA (operator PEMs)
#   - a CONNECT-only egress proxy; the server hostname resolves only via the proxy
#   - CA trust via GIT_SSL_CAINFO; proxy via HTTPS_PROXY
#   - identity + SSH commit signing from a global config file and GIT_CONFIG_COUNT env
# Then: arks clone → corpus workflow → signed commit → push; stock git (peer oracle
# only) re-clones, runs fsck and verify-commit; finally the agent-tier flag corpus runs
# against pin git. Receipt: verify/gates/g042-cloud-dropin/out/receipt.json.
# Outside Docker the same script is HOST ADVISORY only.
set -euo pipefail
cd "$(dirname "$0")/../.."
ROOT="$PWD"
GATE="verify/gates/g042-cloud-dropin"
OUT="$ROOT/$GATE/out"; rm -rf "$OUT"; mkdir -p "$OUT"
for t in git openssl python3 ssh-keygen; do
  command -v "$t" >/dev/null || { echo "G42: $t missing" >&2; exit 1; }
done
echo "G42: stock git $(git --version)"
if [[ -z "${ARKS_BIN:-}" ]]; then
  cargo build --release -p arks
  ARKS_BIN="${CARGO_TARGET_DIR:-target}/release/arks"
fi
ARKS_BIN="$(cd "$(dirname "$ARKS_BIN")" && pwd)/$(basename "$ARKS_BIN")"

WORK="$(mktemp -d /tmp/arks-g42.XXXXXX)"
PIDS=()
cleanup() { for p in "${PIDS[@]}"; do kill "$p" 2>/dev/null || true; done; rm -rf "$WORK"; }
trap cleanup EXIT
free_port() { python3 -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1])'; }
wait_port() { python3 - "$1" <<'PY'
import socket, sys, time
for _ in range(100):
    try:
        socket.create_connection(("127.0.0.1", int(sys.argv[1])), 0.2).close(); sys.exit(0)
    except OSError:
        time.sleep(0.1)
sys.exit(1)
PY
}

HOST=arks-g42.test
# Private CA + server certificate (operator PEMs; product only loads them).
openssl req -x509 -newkey rsa:2048 -nodes -keyout "$WORK/ca.key" -out "$WORK/ca.crt" \
  -days 1 -subj "/CN=arks-g42-ca" >/dev/null 2>&1
openssl req -newkey rsa:2048 -nodes -keyout "$WORK/server.key" -out "$WORK/server.csr" \
  -subj "/CN=$HOST" >/dev/null 2>&1
printf 'subjectAltName=DNS:%s\n' "$HOST" >"$WORK/ext.cnf"
openssl x509 -req -in "$WORK/server.csr" -CA "$WORK/ca.crt" -CAkey "$WORK/ca.key" \
  -CAcreateserial -out "$WORK/server.crt" -days 1 -extfile "$WORK/ext.cnf" >/dev/null 2>&1

# Seed repository (stock git oracle builds the fixture).
bash verify/fixtures/cli-flag-corpus/build-fixture.sh "$WORK/seed" >/dev/null
git -C "$WORK/seed" stash -q -u
git clone -q --bare "$WORK/seed" "$WORK/repo.git"
git -C "$WORK/repo.git" config http.receivepack true

SPORT="$(free_port)"; PPORT="$(free_port)"
"$ARKS_BIN" serve --no-ssh --no-forge --listen "127.0.0.1:$SPORT" \
  --tls-cert "$WORK/server.crt" --tls-key "$WORK/server.key" \
  "$WORK/repo.git" >"$WORK/server.out" 2>"$WORK/server.err" &
PIDS+=($!)
G42_HOSTS="$HOST=127.0.0.1:$SPORT" python3 "$GATE/connect-proxy.py" "$PPORT" "$WORK/proxy.log" &
PIDS+=($!)
wait_port "$SPORT"; wait_port "$PPORT"
URL="https://$HOST/repo.git"

# Sandbox-shaped client environment.
ssh-keygen -q -t ed25519 -N '' -C g42 -f "$WORK/signing_key"
cat >"$WORK/gitconfig" <<CFG
[user]
	name = G42 Agent
	email = g42@example.invalid
	signingkey = $WORK/signing_key.pub
[gpg]
	format = ssh
[gpg "ssh"]
	program = ssh-keygen
	allowedSignersFile = $WORK/allowed_signers
[commit]
	gpgsign = true
CFG
printf 'g42@example.invalid %s\n' "$(cat "$WORK/signing_key.pub")" >"$WORK/allowed_signers"
eval "$(ssh-agent -s)" >/dev/null; PIDS+=("$SSH_AGENT_PID"); ssh-add -q "$WORK/signing_key"
export GIT_CONFIG_GLOBAL="$WORK/gitconfig" GIT_CONFIG_NOSYSTEM=1
export GIT_SSL_CAINFO="$WORK/ca.crt" HTTPS_PROXY="http://127.0.0.1:$PPORT" https_proxy="http://127.0.0.1:$PPORT" NO_PROXY= no_proxy=
unset ALL_PROXY all_proxy SSL_CERT_FILE
export GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=push.default GIT_CONFIG_VALUE_0=current
export ARKS_PERSONALITY=git

fails=0; rows=()
must() {
  local id="$1"; shift
  if "$@" >"$OUT/$id.out" 2>"$OUT/$id.err"; then rows+=("\"$id\":\"PASS\"")
  else rows+=("\"$id\":\"FAIL\""); fails=$((fails + 1)); echo "G42: FAIL $id" >&2; fi
}
cd "$WORK"
must T-CLONE-VIA-PROXY "$ARKS_BIN" clone "$URL" client
must T-PROXY-USED grep -q "CONNECT $HOST:443" proxy.log
cd client || { echo "G42: clone failed" >&2; exit 1; }
must C-IDENTITY-CONFIG test "$("$ARKS_BIN" config user.name)" = "G42 Agent"
must W-BRANCH "$ARKS_BIN" checkout -b g42-topic
printf 'g42 change\n' >>README.md
must W-COMMIT-AM "$ARKS_BIN" commit -am "g42: signed change"
must S-GPGSIG sh -c 'git cat-file -p HEAD | grep -q "^gpgsig "'
must S-VERIFY-COMMIT git verify-commit HEAD
must C-AUTHOR test "$(git log -1 --format='%an <%ae>')" = "G42 Agent <g42@example.invalid>"
must W-PUSH-U "$ARKS_BIN" push -u origin g42-topic
must W-FETCH-HEAD sh -c "'$ARKS_BIN' fetch origin main && test -s .git/FETCH_HEAD"
must W-MERGE "$ARKS_BIN" merge --no-edit FETCH_HEAD
cd "$WORK"
must O-PEER-CLONE git clone -q -b g42-topic "$URL" peer
must O-PEER-FSCK git -C peer fsck --no-progress
must O-PEER-VERIFY git -C peer verify-commit HEAD
must O-PEER-TIP test "$(git -C peer rev-parse HEAD)" = "$(git -C client rev-parse HEAD)"
cd "$ROOT"

# Agent-tier flag corpus vs pin git (the clean-room run of C7a).
CORPUS_OUT="$OUT/corpus"
(unset GIT_CONFIG_GLOBAL GIT_CONFIG_COUNT GIT_CONFIG_KEY_0 GIT_CONFIG_VALUE_0 HTTPS_PROXY https_proxy
 bash verify/scripts/run-cli-flag-corpus-compare.sh "$ARKS_BIN" \
   verify/fixtures/cli-flag-corpus/agent-tier-a.txt "$CORPUS_OUT" >/dev/null) || true
corpus="$(cat "$CORPUS_OUT/summary.json" 2>/dev/null || echo '{}')"

{
  printf '{\n  "gate": "g042-cloud-dropin",\n  "git": "%s",\n  "tip": "%s",\n' \
    "$(git --version)" "$(git -C "$ROOT" rev-parse --short HEAD 2>/dev/null || echo unknown)"
  printf '  "must_failures": %d,\n  "rows": {%s},\n  "corpus": %s\n}\n' \
    "$fails" "$(IFS=,; echo "${rows[*]}")" "$corpus"
} >"$OUT/receipt.json"
cat "$OUT/receipt.json"
[[ $fails -eq 0 ]] && echo "G42: must rows PASS (corpus SAME rate recorded, not gated)"
exit $(( fails > 0 ))
