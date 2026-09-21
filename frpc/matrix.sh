#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"

peers=()
for entry in *-peer/; do
  name="${entry%/}"
  if [[ "$name" != "skew-peer" && -x "$name/run.sh" ]]; then
    peers+=("$name")
  fi
done

if [[ ${#peers[@]} -lt 2 ]]; then
  echo "need at least 2 peers with a run.sh, found: ${peers[*]:-none}" >&2
  exit 1
fi

echo "peers: ${peers[*]}"
fail=0

start_server() {
  SERVER_FIFO="$(mktemp -u)"
  SERVER_LOG="$(mktemp)"
  mkfifo "$SERVER_FIFO"
  "$1/run.sh" serve 127.0.0.1:0 < "$SERVER_FIFO" > "$SERVER_LOG" 2>&1 &
  SERVER_PID=$!
  exec 3> "$SERVER_FIFO"
  SERVER_ADDR=""
  for _ in $(seq 100); do
    SERVER_ADDR="$(sed -n 's/^listening //p' "$SERVER_LOG" | head -n 1)"
    if [[ -n "$SERVER_ADDR" ]]; then
      return 0
    fi
    sleep 0.1
  done
  echo "--- $1 did not start ---"
  cat "$SERVER_LOG"
  return 1
}

stop_server() {
  exec 3>&-
  wait "$SERVER_PID" || true
  rm -f "$SERVER_FIFO" "$SERVER_LOG"
}

report() {
  if [[ "$1" -eq 0 ]]; then
    echo "PASS: $2"
  else
    echo "FAIL: $2"
    echo "$3"
    fail=1
  fi
}

reference="${peers[0]}"
expected="$("$reference/run.sh" frames)"
for peer in "${peers[@]}"; do
  actual="$("$peer/run.sh" frames)"
  status=0
  [[ "$actual" == "$expected" ]] || status=1
  report "$status" "frames $peer = $reference" "$(diff <(echo "$expected") <(echo "$actual") || true)"
done

for server in "${peers[@]}"; do
  for client in "${peers[@]}"; do
    if ! start_server "$server"; then
      report 1 "$server (server) x $client (client)" "server did not start"
      continue
    fi
    output="$("$client/run.sh" drive "$SERVER_ADDR" 2>&1 || true)"
    status=0
    [[ "$output" == *"all checks passed"* ]] || status=1
    report "$status" "$server (server) x $client (client)" "$output"
    stop_server
  done
done

for peer in "${peers[@]}"; do
  if start_server skew-peer; then
    output="$("$peer/run.sh" say "$SERVER_ADDR" "xin chao" 2>&1 || true)"
    status=0
    [[ "$output" == "XIN CHAO" ]] || status=1
    report "$status" "skew-peer (server, field appended) x $peer (client)" "$output"
    stop_server
  else
    report 1 "skew-peer (server) x $peer (client)" "server did not start"
  fi

  if start_server "$peer"; then
    output="$(skew-peer/run.sh say "$SERVER_ADDR" "xin chao" 2>&1 || true)"
    status=0
    [[ "$output" == "XIN CHAO" ]] || status=1
    report "$status" "$peer (server) x skew-peer (client, field appended)" "$output"
    stop_server
  else
    report 1 "$peer (server) x skew-peer (client)" "server did not start"
  fi
done

exit $fail
