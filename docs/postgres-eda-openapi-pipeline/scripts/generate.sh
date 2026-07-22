#!/usr/bin/env bash

set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/_common.sh"

if [[ $# -ne 1 ]]; then
  echo "usage: $0 <version>" >&2
  exit 2
fi

version_id="$(resolve_version "$1" id)"
python_bin="${PYTHON:-python3}"
validator=""
if [[ -x "$PIPELINE_ROOT/.venv/bin/python" ]]; then
  python_bin="$PIPELINE_ROOT/.venv/bin/python"
fi
if [[ -x "$PIPELINE_ROOT/.venv/bin/openapi-spec-validator" ]]; then
  validator="$PIPELINE_ROOT/.venv/bin/openapi-spec-validator"
elif command -v openapi-spec-validator >/dev/null 2>&1; then
  validator="$(command -v openapi-spec-validator)"
fi

"$python_bin" "$PIPELINE_ROOT/tools/generate_openapi.py" "$version_id"
spec="$PIPELINE_ROOT/openapi/$version_id/postgres.openapi.yaml"
if [[ -n "$validator" ]]; then
  "$validator" "$spec"
else
  echo "warning: openapi-spec-validator is not installed; skipped schema validation" >&2
fi
