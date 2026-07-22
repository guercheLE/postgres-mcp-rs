#!/usr/bin/env bash

set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/_common.sh"

if [[ $# -ne 1 ]]; then
  echo "usage: $0 <version>" >&2
  exit 2
fi

require_docker
version_id="$(resolve_version "$1" id)"
container="$(resolve_version "$1" container)"
require_running_container "$container"

output_dir="$PIPELINE_ROOT/data/$version_id"
mkdir -p "$output_dir"

for dataset in metadata routines relations types; do
  output="$output_dir/$dataset.json"
  temporary="$output.tmp"
  echo "extracting $dataset from $container"
  docker exec -i "$container" \
    psql -XqAt --no-psqlrc --set ON_ERROR_STOP=1 --username postgres --dbname eda \
    < "$PIPELINE_ROOT/sql/eda/$dataset.sql" > "$temporary"
  python3 -m json.tool "$temporary" >/dev/null
  mv "$temporary" "$output"
done

echo "wrote EDA data to $output_dir"
