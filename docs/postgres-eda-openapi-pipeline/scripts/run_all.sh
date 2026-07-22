#!/usr/bin/env bash

set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/_common.sh"

refresh=false
stable_only=false
keep_containers=false
for argument in "$@"; do
  case "$argument" in
    --refresh) refresh=true ;;
    --stable-only) stable_only=true ;;
    --keep-containers) keep_containers=true ;;
    *) echo "usage: $0 [--refresh] [--stable-only] [--keep-containers]" >&2; exit 2 ;;
  esac
done

if [[ "$refresh" == true ]]; then
  "$PIPELINE_ROOT/scripts/refresh_versions.sh"
fi

list_args=(list)
if [[ "$stable_only" == true ]]; then
  list_args+=(--stable-only)
fi

current_version=""
cleanup() {
  if [[ -n "$current_version" && "$keep_containers" != true ]]; then
    "$PIPELINE_ROOT/scripts/down.sh" "$current_version" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT INT TERM

version_list="$(python3 "$VERSION_TOOL" "${list_args[@]}")"
if [[ -z "$version_list" ]]; then
  echo "error: the version manifest returned no PostgreSQL versions" >&2
  exit 1
fi

while IFS= read -r version; do
  current_version="$version"
  echo "== PostgreSQL $version =="
  "$PIPELINE_ROOT/scripts/up.sh" "$version"
  "$PIPELINE_ROOT/scripts/extract.sh" "$version"
  "$PIPELINE_ROOT/scripts/generate.sh" "$version"
  if [[ "$keep_containers" != true ]]; then
    "$PIPELINE_ROOT/scripts/down.sh" "$version"
  fi
  current_version=""
done <<< "$version_list"

echo "completed every configured PostgreSQL version"
