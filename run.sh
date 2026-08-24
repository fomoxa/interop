#!/usr/bin/env bash
# Pairs every peer under this directory against every other peer (including
# itself), both as server and as client, and fails if any pair fails.
#
# A peer is any directory with an executable run.sh that accepts:
#   run.sh --role server|client --addr host:port
# and exits 0 once it has completed the Player.edge exchange, non-zero
# otherwise. Adding a new language means adding a new peer directory with
# that run.sh - nothing here has to change.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"

peers=()
for entry in */; do
  name="${entry%/}"
  if [[ -x "$name/run.sh" ]]; then
    peers+=("$name")
  fi
done

if [[ ${#peers[@]} -lt 2 ]]; then
  echo "need at least 2 peers with a run.sh, found: ${peers[*]:-none}" >&2
  exit 1
fi

echo "peers: ${peers[*]}"

fail=0
port=20000

for transport in tcp udp; do
  for server in "${peers[@]}"; do
    for client in "${peers[@]}"; do
      port=$((port + 1))
      addr="127.0.0.1:$port"
      server_log="$(mktemp)"
      client_log="$(mktemp)"

      echo "=== [$transport] $server (server) x $client (client) @ $addr ==="

      "$server/run.sh" --role server --addr "$addr" --transport "$transport" > "$server_log" 2>&1 &
      server_pid=$!
      sleep 1

      client_ok=1
      "$client/run.sh" --role client --addr "$addr" --transport "$transport" > "$client_log" 2>&1 || client_ok=0

      server_ok=1
      wait "$server_pid" || server_ok=0

      if [[ $server_ok -eq 1 && $client_ok -eq 1 ]]; then
        echo "PASS: [$transport] $server(server) x $client(client)"
      else
        echo "FAIL: [$transport] $server(server) x $client(client)"
        echo "--- server log ---"; cat "$server_log"
        echo "--- client log ---"; cat "$client_log"
        fail=1
      fi

      rm -f "$server_log" "$client_log"
    done
  done
done

exit $fail
