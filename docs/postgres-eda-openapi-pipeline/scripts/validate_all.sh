#!/usr/bin/env bash

set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/_common.sh"

validator="$PIPELINE_ROOT/.venv/bin/openapi-spec-validator"
if [[ ! -x "$validator" ]]; then
  echo "error: install dependencies with python3 -m venv .venv && .venv/bin/pip install -r tools/requirements.txt" >&2
  exit 1
fi

version_list="$(python3 "$VERSION_TOOL" list)"
if [[ -z "$version_list" ]]; then
  echo "error: the version manifest returned no PostgreSQL versions" >&2
  exit 1
fi

while IFS= read -r version; do
  spec="$PIPELINE_ROOT/openapi/$version/postgres.openapi.yaml"
  [[ -f "$spec" ]] || { echo "error: missing $spec" >&2; exit 1; }
  "$validator" "$spec"
done <<< "$version_list"
