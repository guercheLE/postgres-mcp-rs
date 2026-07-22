#!/usr/bin/env bash

set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/_common.sh"

if [[ $# -ne 1 ]]; then
  echo "usage: $0 <version>" >&2
  exit 2
fi

require_docker
require_password

version_id="$(resolve_version "$1" id)"
release="$(resolve_version "$1" release)"
image="$(resolve_version "$1" image)"
container="$(resolve_version "$1" container)"

if docker inspect "$container" >/dev/null 2>&1; then
  require_owned_container "$container"
  container_release="$(
    docker inspect -f '{{ index .Config.Labels "postgres-eda-release" }}' "$container"
  )"
  if [[ "$container_release" == "$release" && \
        "$(docker inspect -f '{{.State.Running}}' "$container")" == "true" ]]; then
    echo "$container is already running"
    exit 0
  fi
  docker rm -f "$container" >/dev/null
fi

echo "starting PostgreSQL $version_id from $image"
docker run --detach --pull=always \
  --name "$container" \
  --label postgres-eda-openapi-pipeline=true \
  --label "postgres-eda-version=$version_id" \
  --label "postgres-eda-release=$release" \
  --env "POSTGRES_PASSWORD=$POSTGRES_PASSWORD" \
  --env POSTGRES_DB=eda \
  "$image" >/dev/null

for attempt in $(seq 1 120); do
  server_version=""
  if server_version="$(
    docker exec "$container" psql -XAtq -U postgres -d eda -c 'SHOW server_version' 2>/dev/null
  )" && [[ -n "$server_version" ]]; then
    if [[ "$server_version" != "$release" ]]; then
      echo "error: $image reported PostgreSQL $server_version; expected $release" >&2
      exit 1
    fi
    echo "$container is ready (PostgreSQL $server_version)"
    exit 0
  fi
  if [[ "$(docker inspect -f '{{.State.Running}}' "$container" 2>/dev/null || true)" != "true" ]]; then
    docker logs "$container" >&2
    echo "error: $container stopped during startup" >&2
    exit 1
  fi
  sleep 1
done

docker logs "$container" >&2
echo "error: timed out waiting for $container" >&2
exit 1
