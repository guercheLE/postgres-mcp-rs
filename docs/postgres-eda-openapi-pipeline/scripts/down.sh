#!/usr/bin/env bash

set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/_common.sh"

require_docker

if [[ $# -eq 1 && "$1" == "--all" ]]; then
  containers="$(docker ps -aq --filter label=postgres-eda-openapi-pipeline=true)"
  if [[ -n "$containers" ]]; then
    # The IDs come only from this pipeline's explicit Docker label.
    docker rm -f $containers
  fi
  exit 0
fi

if [[ $# -ne 1 ]]; then
  echo "usage: $0 <version> | --all" >&2
  exit 2
fi

container="$(resolve_version "$1" container)"
if docker inspect "$container" >/dev/null 2>&1; then
  require_owned_container "$container"
  docker rm -f "$container" >/dev/null
  echo "removed $container"
else
  echo "$container does not exist"
fi
