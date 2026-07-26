use std::path::{Path, PathBuf};

use postgres_mcp::data::store::VERSION_STORE_FILES;
use serde_json::Value;

fn assert_self_contained_refs(root: &Value, value: &Value, context: &str) {
    match value {
        Value::Object(map) => {
            if let Some(reference) = map.get("$ref").and_then(Value::as_str) {
                assert!(
                    reference.starts_with("#/$defs/"),
                    "{context}: unresolved external component reference '{reference}'"
                );
                let pointer = reference.strip_prefix('#').unwrap();
                assert!(
                    root.pointer(pointer).is_some(),
                    "{context}: local reference '{reference}' has no embedded definition"
                );
            }
            if let Some(reference) = map.get("$dynamicRef").and_then(Value::as_str) {
                assert!(
                    reference.starts_with("#/$defs/"),
                    "{context}: unresolved external dynamic reference '{reference}'"
                );
                let pointer = reference.strip_prefix('#').unwrap();
                assert!(
                    root.pointer(pointer).is_some(),
                    "{context}: local dynamic reference '{reference}' has no embedded definition"
                );
            }
            for child in map.values() {
                assert_self_contained_refs(root, child, context);
            }
        }
        Value::Array(values) => {
            for child in values {
                assert_self_contained_refs(root, child, context);
            }
        }
        _ => {}
    }
}

fn compressed_store_path(file: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("{file}.zst"))
}

#[test]
fn every_stored_input_and_output_schema_is_self_contained() {
    for (version, file) in VERSION_STORE_FILES {
        let compressed = std::fs::read(compressed_store_path(file)).unwrap();
        let raw = zstd::decode_all(compressed.as_slice()).unwrap();
        let temp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(temp.path(), raw).unwrap();
        let connection = rusqlite::Connection::open(temp.path()).unwrap();
        let mut statement = connection
            .prepare("SELECT operation_id, input_schema, output_schema FROM endpoints")
            .unwrap();
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .unwrap();
        for row in rows {
            let (operation_id, input, output) = row.unwrap();
            for (kind, serialized) in [("input", input), ("output", output)] {
                let schema: Value = serde_json::from_str(&serialized).unwrap_or_else(|error| {
                    panic!("version {version} operation {operation_id} has invalid {kind} JSON: {error}")
                });
                let context = format!("version {version} operation {operation_id} {kind}");
                assert_self_contained_refs(&schema, &schema, &context);
            }
        }
    }
}

fn assert_no_refs(value: &Value, context: &str) {
    match value {
        Value::Object(map) => {
            assert!(
                !map.contains_key("$ref") && !map.contains_key("$dynamicRef"),
                "{context}: expected every $ref to be fully inlined, found one"
            );
            for child in map.values() {
                assert_no_refs(child, context);
            }
        }
        Value::Array(values) => {
            for child in values {
                assert_no_refs(child, context);
            }
        }
        _ => {}
    }
}

/// No PostgreSQL catalog operation's schema is genuinely self-referential
/// (unlike, say, a generic tree-shaped REST resource), so mcpify's
/// `schema_resolve::finalize_with_defs` can always fully inline every
/// `$ref` here — this asserts that actually happens, not just that any
/// remaining `$ref` would be self-contained.
#[test]
fn every_stored_input_and_output_schema_has_no_ref_left() {
    for (version, file) in VERSION_STORE_FILES {
        let compressed = std::fs::read(compressed_store_path(file)).unwrap();
        let raw = zstd::decode_all(compressed.as_slice()).unwrap();
        let temp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(temp.path(), raw).unwrap();
        let connection = rusqlite::Connection::open(temp.path()).unwrap();
        let mut statement = connection
            .prepare("SELECT operation_id, input_schema, output_schema FROM endpoints")
            .unwrap();
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .unwrap();
        for row in rows {
            let (operation_id, input, output) = row.unwrap();
            for (kind, serialized) in [("input", input), ("output", output)] {
                let schema: Value = serde_json::from_str(&serialized).unwrap();
                let context = format!("version {version} operation {operation_id} {kind}");
                assert_no_refs(&schema, &context);
            }
        }
    }
}

#[test]
fn every_validation_schema_is_self_contained() {
    for (version, file) in VERSION_STORE_FILES {
        let suffix = file
            .strip_prefix("mcp_store")
            .and_then(|value| value.strip_suffix(".db"))
            .unwrap();
        let name = if suffix.is_empty() {
            "generated_schemas.json.zst".to_string()
        } else {
            format!("generated_schemas{suffix}.json.zst")
        };
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/validation")
            .join(name);
        let compressed = std::fs::read(&path).unwrap();
        let raw = zstd::decode_all(compressed.as_slice()).unwrap();
        let operations: Value = serde_json::from_slice(&raw).unwrap();
        for (operation_id, schemas) in operations.as_object().unwrap() {
            for key in ["inputSchema", "outputSchema"] {
                let schema = &schemas[key];
                let context = format!("version {version} operation {operation_id} {key}");
                assert_self_contained_refs(schema, schema, &context);
            }
        }
    }
}
