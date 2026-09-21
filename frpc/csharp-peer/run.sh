#!/usr/bin/env bash
set -euo pipefail
dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec dotnet "$dir/bin/Release/net8.0/frpc-csharp-peer.dll" "$@"
