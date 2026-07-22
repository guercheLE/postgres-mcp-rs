#!/usr/bin/env python3
"""Generate a synthetic OpenAPI 3.1 catalog from PostgreSQL EDA JSON."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import sys
from typing import Any

import yaml


ROOT = Path(__file__).resolve().parent.parent


SCALAR_TYPES: dict[str, tuple[str, str | None]] = {
    "smallint": ("integer", "int32"),
    "integer": ("integer", "int32"),
    "bigint": ("integer", "int64"),
    "oid": ("integer", "int64"),
    "real": ("number", "float"),
    "double precision": ("number", "double"),
    "numeric": ("number", "double"),
    "decimal": ("number", "double"),
    "money": ("number", "double"),
    "boolean": ("boolean", None),
    "bool": ("boolean", None),
    "date": ("string", "date"),
    "timestamp without time zone": ("string", "date-time"),
    "timestamp with time zone": ("string", "date-time"),
    "time without time zone": ("string", "time"),
    "time with time zone": ("string", "time"),
    "uuid": ("string", "uuid"),
    "bytea": ("string", "byte"),
    "json": ("object", None),
    "jsonb": ("object", None),
    "void": ("null", None),
}


def load_json(path: Path) -> Any:
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)


def normalize_type_name(data_type: str) -> str:
    value = re.sub(r"\s+", " ", data_type.strip().lower())
    value = re.sub(r"\([^)]*\)$", "", value).strip()
    return value


def postgres_type_to_schema(data_type: str) -> dict:
    exact = data_type.strip()
    normalized = normalize_type_name(exact)
    if normalized.endswith("[]"):
        item_type = exact[: exact.rfind("[]")]
        return {
            "type": "array",
            "items": postgres_type_to_schema(item_type) | {"x-postgres-type": item_type.strip()},
            "x-postgres-type": exact,
        }
    if normalized in ("json", "jsonb"):
        return {"x-postgres-type": exact}
    openapi_type, openapi_format = SCALAR_TYPES.get(normalized, ("string", None))
    schema: dict[str, Any] = {"type": openapi_type}
    if openapi_format:
        schema["format"] = openapi_format
    schema["x-postgres-type"] = exact
    return schema


def slug(value: str) -> str:
    result = re.sub(r"[^a-z0-9]+", "_", value.lower()).strip("_")
    return result[:48] or "none"


def human_text(value: str) -> str:
    """Render catalog identifiers as natural-language text for semantic search."""
    return value.replace("_", " ")


def signature_token(identity_arguments: str) -> str:
    digest = hashlib.sha256(identity_arguments.encode("utf-8")).hexdigest()[:8]
    return f"{slug(identity_arguments)}_{digest}"


def unique_operation_id(prefix: str, schema_name: str, object_name: str, discriminator: str) -> str:
    """Build a readable operationId while keeping its uniqueness suffix intact."""
    digest = hashlib.sha256(discriminator.encode("utf-8")).hexdigest()[:10]
    return "_".join((slug(prefix)[:12], slug(schema_name)[:20], slug(object_name)[:36], digest))


def error_responses() -> dict:
    return {
        code: {
            "description": description,
            "content": {"application/json": {"schema": {"$ref": "#/components/schemas/PostgresError"}}},
        }
        for code, description in {
            "400": "Invalid SQL argument or data exception.",
            "403": "Insufficient PostgreSQL privilege.",
            "409": "Constraint, serialization, or transaction conflict.",
            "500": "PostgreSQL server or operator intervention error.",
        }.items()
    }


def allow_null(schema: dict) -> dict:
    result = dict(schema)
    schema_type = result.get("type")
    if schema_type == "null":
        return result
    if isinstance(schema_type, str):
        result["type"] = [schema_type, "null"]
    elif isinstance(schema_type, list) and "null" not in schema_type:
        result["type"] = [*schema_type, "null"]
    # A schema without `type` (notably json/jsonb) already accepts null.
    return result


def argument_schema(argument: dict) -> dict:
    schema = postgres_type_to_schema(argument["data_type"])
    if argument.get("mode") == "variadic":
        if schema["type"] == "array":
            return allow_null(schema)
        schema = {"type": "array", "items": schema, "x-postgres-type": argument["data_type"]}
    return allow_null(schema)


def argument_name(argument: dict) -> str:
    return argument.get("name") or f"arg{argument.get('ordinal', 1)}"


def object_schema(columns: list[dict]) -> dict:
    properties = {}
    required = []
    for column in sorted(columns, key=lambda item: item.get("ordinal", 0)):
        property_schema = postgres_type_to_schema(column["data_type"])
        if column.get("nullable", True):
            property_schema = allow_null(property_schema)
        properties[column["name"]] = property_schema
        required.append(column["name"])
    schema: dict[str, Any] = {"type": "object", "properties": properties}
    if required:
        schema["required"] = required
    return schema


def routine_response_schema(routine: dict) -> dict:
    output_arguments = [
        item for item in routine.get("arguments", []) if item.get("mode") in ("out", "inout", "table")
    ]
    columns = routine.get("return_columns") or []
    if output_arguments:
        row = {
            "type": "object",
            "properties": {argument_name(item): argument_schema(item) for item in output_arguments},
        }
    elif columns:
        row = object_schema(columns)
    else:
        row = postgres_type_to_schema(routine["result_type"])
    if routine.get("returns_set"):
        return {"type": "array", "items": allow_null(row)}
    return allow_null(row)


def routine_operation(routine: dict) -> tuple[str, dict]:
    schema_name = routine["schema_name"]
    routine_name = routine["routine_name"]
    signature = signature_token(routine.get("identity_arguments", ""))
    operation_id = unique_operation_id(
        "call",
        schema_name,
        routine_name,
        f"{schema_name}.{routine_name}({routine.get('identity_arguments', '')})",
    )
    path = f"/routines/{schema_name}/{routine_name}/{signature}"
    input_arguments = [
        item for item in routine.get("arguments", []) if item.get("mode") in ("in", "inout", "variadic")
    ]
    request_schema: dict[str, Any] = {
        "type": "object",
        "properties": {argument_name(item): argument_schema(item) for item in input_arguments},
    }
    required = [argument_name(item) for item in input_arguments if not item.get("has_default", False)]
    if required:
        request_schema["required"] = required

    description = human_text(routine.get("description") or (
        f"PostgreSQL {routine['routine_kind']} {schema_name}.{routine_name}"
    ))
    operation: dict[str, Any] = {
        "operationId": operation_id,
        "summary": description.splitlines()[0][:180],
        "description": description,
        "tags": [schema_name, "routines"],
        "x-postgres-schema": schema_name,
        "x-postgres-routine-kind": routine["routine_kind"],
        "x-postgres-identity-arguments": routine.get("identity_arguments", ""),
        "requestBody": {
            "required": bool(required),
            "content": {"application/json": {"schema": request_schema}},
        },
        "responses": {
            "200": {
                "description": "Routine result.",
                "content": {"application/json": {"schema": routine_response_schema(routine)}},
            },
            **error_responses(),
        },
    }
    return path, {"post": operation}


def relation_operation(relation: dict) -> tuple[str, dict]:
    schema_name = relation["schema_name"]
    relation_name = relation["relation_name"]
    operation_id = unique_operation_id(
        "read", schema_name, relation_name, f"{schema_name}.{relation_name}"
    )
    path = f"/relations/{schema_name}/{relation_name}"
    row_schema = object_schema(relation.get("columns", []))
    description = human_text(relation.get("description") or (
        f"Read PostgreSQL {relation['relation_kind']} {schema_name}.{relation_name}."
    ))
    operation = {
        "operationId": operation_id,
        "summary": description.splitlines()[0][:180],
        "description": description,
        "tags": [schema_name, "relations"],
        "x-postgres-schema": schema_name,
        "x-postgres-relation-kind": relation["relation_kind"],
        "parameters": [
            {
                "name": "limit",
                "in": "query",
                "schema": {"type": "integer", "minimum": 1, "maximum": 10000, "default": 100},
                "description": "Maximum rows to return.",
            },
            {
                "name": "offset",
                "in": "query",
                "schema": {"type": "integer", "minimum": 0, "default": 0},
                "description": "Rows to skip before returning results.",
            },
        ],
        "responses": {
            "200": {
                "description": "Relation rows.",
                "content": {"application/json": {"schema": {"type": "array", "items": row_schema}}},
            },
            **error_responses(),
        },
    }
    return path, {"get": operation}


def validate_spec_contract(spec: dict) -> None:
    operation_ids = set()
    duplicate_operation_ids = set()
    prose_with_underscores = []

    def inspect(value: Any, location: str = "$") -> None:
        if isinstance(value, dict):
            for key, child in value.items():
                child_location = f"{location}.{key}"
                if key in ("summary", "description") and isinstance(child, str) and "_" in child:
                    prose_with_underscores.append(child_location)
                inspect(child, child_location)
        elif isinstance(value, list):
            for index, child in enumerate(value):
                inspect(child, f"{location}[{index}]")

    for path_item in spec["paths"].values():
        for operation in path_item.values():
            operation_id = operation["operationId"]
            if operation_id in operation_ids:
                duplicate_operation_ids.add(operation_id)
            operation_ids.add(operation_id)
    if duplicate_operation_ids:
        raise ValueError(
            f"duplicate operationId values: {', '.join(sorted(duplicate_operation_ids))}"
        )

    inspect(spec)
    if prose_with_underscores:
        raise ValueError(
            "summary/description values contain underscores at: " + ", ".join(prose_with_underscores)
        )


def build_spec(
    version_id: str,
    image: str,
    metadata: dict,
    routines: list[dict],
    relations: list[dict],
) -> dict:
    paths: dict[str, Any] = {}
    for routine in routines:
        path, item = routine_operation(routine)
        if path in paths:
            raise ValueError(f"duplicate routine path: {path}")
        paths[path] = item
    for relation in relations:
        path, item = relation_operation(relation)
        if path in paths:
            raise ValueError(f"duplicate relation path: {path}")
        paths[path] = item

    server_version = metadata.get("server_version", version_id)
    spec = {
        "openapi": "3.1.0",
        "info": {
            "title": f"PostgreSQL {server_version} catalog",
            "version": str(server_version),
            "description": human_text(
                "Synthetic OpenAPI representation generated from live pg_catalog and "
                "information_schema introspection. Paths describe SQL routines and readable "
                "relations; they are not an HTTP endpoint exposed by PostgreSQL itself."
            ),
        },
        "servers": [
            {
                "url": "postgresql://{host}:{port}/{database}",
                "variables": {
                    "host": {"default": "localhost"},
                    "port": {"default": "5432"},
                    "database": {"default": "eda"},
                },
            }
        ],
        "security": [{"postgresPassword": []}],
        "paths": dict(sorted(paths.items())),
        "components": {
            "securitySchemes": {
                "postgresPassword": {
                    "type": "http",
                    "scheme": "basic",
                    "description": (
                        "Synthetic representation of PostgreSQL username/password credentials. "
                        "The actual connection uses the PostgreSQL wire protocol, not HTTP Basic."
                    ),
                }
            },
            "schemas": {
                "PostgresError": {
                    "type": "object",
                    "properties": {
                        "sqlstate": {"type": "string", "pattern": "^[0-9A-Z]{5}$"},
                        "severity": {"type": "string"},
                        "message": {"type": "string"},
                        "detail": {"type": ["string", "null"]},
                        "hint": {"type": ["string", "null"]},
                        "position": {"type": ["integer", "null"]},
                        "where": {"type": ["string", "null"]},
                        "schema": {"type": ["string", "null"]},
                        "table": {"type": ["string", "null"]},
                        "column": {"type": ["string", "null"]},
                        "dataType": {"type": ["string", "null"]},
                        "constraint": {"type": ["string", "null"]},
                    },
                    "required": ["sqlstate", "message"],
                }
            },
        },
        "x-postgres-eda": {
            "version-id": version_id,
            "server-version": server_version,
            "server-version-num": metadata.get("server_version_num"),
            "docker-image": image,
            "database": metadata.get("database", "eda"),
            "routine-count": len(routines),
            "relation-count": len(relations),
            "schemas": metadata.get("schemas", []),
            "extensions": metadata.get("extensions", []),
        },
    }
    validate_spec_contract(spec)
    return spec


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("version", help="version ID from versions.json")
    args = parser.parse_args()

    from version_manifest import load_manifest, resolve

    version = resolve(load_manifest(), args.version)
    data_dir = ROOT / "data" / version["id"]
    metadata = load_json(data_dir / "metadata.json")
    routines = load_json(data_dir / "routines.json")
    relations = load_json(data_dir / "relations.json")
    spec = build_spec(version["id"], version["image"], metadata, routines, relations)

    out_dir = ROOT / "openapi" / version["id"]
    out_dir.mkdir(parents=True, exist_ok=True)
    out_path = out_dir / "postgres.openapi.yaml"
    temporary_path = out_dir / f".{out_path.name}.tmp"
    try:
        with temporary_path.open("w", encoding="utf-8") as handle:
            yaml.safe_dump(spec, handle, sort_keys=False, width=110, allow_unicode=True)
        temporary_path.replace(out_path)
    finally:
        temporary_path.unlink(missing_ok=True)
    print(f"wrote {out_path} ({len(spec['paths'])} paths)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
