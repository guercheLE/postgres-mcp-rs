#!/usr/bin/env bash

set -euo pipefail

PIPELINE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION_TOOL="$PIPELINE_ROOT/tools/version_manifest.py"

if [[ -f "$PIPELINE_ROOT/.env" ]]; then
  set -a
  # shellcheck disable=SC1091
  source "$PIPELINE_ROOT/.env"
  set +a
fi

resolve_version() {
  python3 "$VERSION_TOOL" resolve "$1" "${2:-id}"
}

require_docker() {
  command -v docker >/dev/null 2>&1 || {
    echo "error: docker is required" >&2
    exit 1
  }
  docker info >/dev/null 2>&1 || {
    echo "error: Docker daemon is not available" >&2
    exit 1
  }
}

require_password() {
  if [[ -z "${POSTGRES_PASSWORD:-}" ]]; then
    echo "error: POSTGRES_PASSWORD is not set; copy .env.example to .env" >&2
    exit 1
  fi
}

require_running_container() {
  local container="$1"
  require_owned_container "$container"
  if [[ "$(docker inspect -f '{{.State.Running}}' "$container" 2>/dev/null || true)" != "true" ]]; then
    echo "error: container $container is not running; run scripts/up.sh first" >&2
    exit 1
  fi
}

require_owned_container() {
  local container="$1"
  local owner
  owner="$(
    docker inspect -f '{{ index .Config.Labels "postgres-eda-openapi-pipeline" }}' \
      "$container" 2>/dev/null || true
  )"
  if [[ "$owner" != "true" ]]; then
    echo "error: container name $container is not owned by this pipeline" >&2
    exit 1
  fi
}
