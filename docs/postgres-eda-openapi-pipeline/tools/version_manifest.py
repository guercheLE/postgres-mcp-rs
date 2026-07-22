#!/usr/bin/env python3
"""Refresh, list, and resolve maintained PostgreSQL Docker image versions."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import json
from pathlib import Path
import re
import subprocess
import sys
from urllib.request import Request, urlopen


ROOT = Path(__file__).resolve().parent.parent
MANIFEST_PATH = ROOT / "versions.json"
UPSTREAM_URL = "https://raw.githubusercontent.com/docker-library/postgres/master/versions.json"
PRERELEASE_RE = re.compile(r"(?:alpha|beta|rc|devel)", re.IGNORECASE)


def build_manifest(upstream: dict, refreshed_at: str) -> dict:
    versions = []
    for key, value in upstream.items():
        major = int(value.get("major", key))
        release = str(value["version"])
        prerelease = bool(PRERELEASE_RE.search(release))
        version_id = release if prerelease else str(major)
        versions.append(
            {
                "id": version_id,
                "major": major,
                "release": release,
                "image": f"postgres:{release if prerelease else major}-alpine",
                "prerelease": prerelease,
            }
        )
    versions.sort(key=lambda item: item["major"], reverse=True)
    return {"source": UPSTREAM_URL, "refreshed_at": refreshed_at, "versions": versions}


def load_manifest(path: Path = MANIFEST_PATH) -> dict:
    with path.open(encoding="utf-8") as handle:
        manifest = json.load(handle)
    if not manifest.get("versions"):
        raise ValueError(f"{path} contains no versions")
    return manifest


def resolve(manifest: dict, value: str) -> dict:
    for version in manifest["versions"]:
        aliases = {
            version["id"],
            str(version["major"]),
            version["release"],
            version["image"],
            version["image"].removeprefix("postgres:"),
        }
        if value in aliases:
            return version
    available = ", ".join(item["id"] for item in manifest["versions"])
    raise ValueError(f"unknown PostgreSQL version {value!r}; available: {available}")


def fetch_upstream() -> dict:
    request = Request(UPSTREAM_URL, headers={"User-Agent": "postgres-eda-openapi-pipeline"})
    with urlopen(request, timeout=30) as response:
        return json.load(response)


def verify_images(manifest: dict) -> None:
    failures = []
    for version in manifest["versions"]:
        image = version["image"]
        print(f"verifying {image} ...", file=sys.stderr)
        result = subprocess.run(
            ["docker", "manifest", "inspect", image],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
            text=True,
            check=False,
        )
        if result.returncode:
            failures.append(f"{image}: {result.stderr.strip()}")
    if failures:
        raise RuntimeError("unavailable Docker images:\n" + "\n".join(failures))


def refresh(verify: bool) -> dict:
    now = datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")
    manifest = build_manifest(fetch_upstream(), now)
    if verify:
        verify_images(manifest)
    MANIFEST_PATH.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    return manifest


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    refresh_parser = subparsers.add_parser("refresh", help="refresh from the official image source")
    refresh_parser.add_argument("--verify-images", action="store_true")

    list_parser = subparsers.add_parser("list", help="list configured version IDs")
    list_parser.add_argument("--stable-only", action="store_true")

    resolve_parser = subparsers.add_parser("resolve", help="resolve a version alias")
    resolve_parser.add_argument("version")
    resolve_parser.add_argument(
        "field", choices=("id", "major", "release", "image", "container"), nargs="?", default="id"
    )

    args = parser.parse_args()
    try:
        if args.command == "refresh":
            manifest = refresh(args.verify_images)
            print(" ".join(version["id"] for version in manifest["versions"]))
            return 0

        manifest = load_manifest()
        if args.command == "list":
            for version in manifest["versions"]:
                if not args.stable_only or not version["prerelease"]:
                    print(version["id"])
            return 0

        version = resolve(manifest, args.version)
        if args.field == "container":
            slug = re.sub(r"[^a-z0-9]+", "-", version["id"].lower()).strip("-")
            print(f"postgres-eda-{slug}")
        else:
            print(version[args.field])
        return 0
    except (OSError, ValueError, RuntimeError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
