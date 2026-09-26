#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
phase="${1:-}"
if [[ "$phase" != before && "$phase" != after ]]; then
  echo "Usage: scripts/security-scan.sh before|after" >&2
  exit 2
fi

network="${ZAP_NETWORK:-ferxarp_act03_scan}"
image="ghcr.io/zaproxy/zaproxy:stable"
output="$PWD/docs/evidencias/security"
mkdir -p "$output"

if [[ "$(docker network inspect -f '{{.Internal}}' "$network")" != true ]]; then
  echo "ZAP_NETWORK must be an internal Docker network containing only local FERXARP services" >&2
  exit 2
fi

for target in frontend backend; do
  if [[ "$target" == frontend ]]; then url=http://frontend:3000; else url=http://backend:8000; fi
  for kind in baseline full; do
    if [[ "$kind" == baseline ]]; then
      command=zap-baseline.py
      suffix=""
    else
      command=zap-full-scan.py
      suffix=-active
    fi
    docker run --rm --network "$network" --user root \
      --volume "$PWD/scripts:/zap/act03-scripts:ro" \
      --volume "$output:/zap/wrk" \
      "$image" "$command" -t "$url" -m 1 -T 10 -I -s \
      --hook=/zap/act03-scripts/zap-scope.py \
      -J "zap-${phase}-${target}${suffix}.json"
  done
done
