//! Native PostgreSQL execution for generated catalog operations and the
//! parameterized `execute_sql` fallback.
//!
//! Identifiers and PostgreSQL type names come only from the embedded,
//! generated endpoint store. Caller-controlled values never enter an SQL
//! string: they are sent through PostgreSQL's extended query protocol as
//! separately bound TEXT parameters and explicitly cast by the statement.

use std::str::FromStr;
use std::time::Duration;

use anyhow::{Context, ensure};
use serde_json::{Map, Value, json};
use tokio_postgres::types::{Json, ToSql, Type};
use tokio_postgres::{Client, Config as PostgresConfig, Transaction};
use tokio_postgres_rustls::MakeRustlsConnect;

use crate::auth::request_credentials::RequestCredentials;
use crate::core::config_schema::Config;
use crate::data::store::EndpointRecord;

const DEFAULT_RELATION_LIMIT: u64 = 100;
const MAX_RELATION_LIMIT: u64 = 10_000;
const DEFAULT_SQL_MAX_ROWS: usize = 100;
const MAX_SQL_MAX_ROWS: usize = 10_000;

fn postgres_error(action: &str, error: tokio_postgres::Error) -> anyhow::Error {
    if let Some(database_error) = error.as_db_error() {
        anyhow::anyhow!(
            "{action}: PostgreSQL {}: {}",
            database_error.code().code(),
            database_error.message()
        )
    } else {
        anyhow::anyhow!("{action}: {error}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum EndpointTarget {
    Relation { schema: String, name: String },
    Routine { schema: String, name: String },
}

#[derive(Debug)]
struct BoundText {
    value: Option<String>,
}

impl BoundText {
    fn from_json(value: &Value, array: bool) -> anyhow::Result<Self> {
        let value = if value.is_null() {
            None
        } else if array {
            Some(postgres_array_literal(value)?)
        } else {
            Some(match value {
                Value::String(value) => value.clone(),
                Value::Bool(_) | Value::Number(_) => value.to_string(),
                Value::Array(_) | Value::Object(_) => serde_json::to_string(value)?,
                Value::Null => unreachable!("null handled above"),
            })
        };
        Ok(Self { value })
    }
}

fn postgres_array_literal(value: &Value) -> anyhow::Result<String> {
    let values = value
        .as_array()
        .context("PostgreSQL array arguments must be JSON arrays")?;
    let mut result = String::from("{");
    for (index, item) in values.iter().enumerate() {
        if index > 0 {
            result.push(',');
        }
        match item {
            Value::Null => result.push_str("NULL"),
            Value::Array(_) => result.push_str(&postgres_array_literal(item)?),
            scalar => {
                let text = match scalar {
                    Value::String(value) => value.clone(),
                    Value::Bool(_) | Value::Number(_) => scalar.to_string(),
                    Value::Object(_) => serde_json::to_string(scalar)?,
                    Value::Null | Value::Array(_) => unreachable!(),
                };
                result.push('"');
                for character in text.chars() {
                    if matches!(character, '"' | '\\') {
                        result.push('\\');
                    }
                    result.push(character);
                }
                result.push('"');
            }
        }
    }
    result.push('}');
    Ok(result)
}

fn quote_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

fn parse_endpoint_target(path: &str) -> anyhow::Result<EndpointTarget> {
    let parts = path.trim_matches('/').split('/').collect::<Vec<_>>();
    match parts.as_slice() {
        ["relations", schema, name] if !schema.is_empty() && !name.is_empty() => {
            Ok(EndpointTarget::Relation {
                schema: (*schema).to_string(),
                name: (*name).to_string(),
            })
        }
        ["routines", schema, name, _signature] if !schema.is_empty() && !name.is_empty() => {
            Ok(EndpointTarget::Routine {
                schema: (*schema).to_string(),
                name: (*name).to_string(),
            })
        }
        _ => anyhow::bail!("unsupported generated PostgreSQL endpoint path '{path}'"),
    }
}

fn type_includes(schema: &Value, expected: &str) -> bool {
    match schema.get("type") {
        Some(Value::String(value)) => value == expected,
        Some(Value::Array(values)) => values.iter().any(|value| value.as_str() == Some(expected)),
        _ => false,
    }
}

fn trusted_postgres_type(schema: &Value) -> anyhow::Result<(String, bool)> {
    let raw = schema
        .get("x-postgres-type")
        .and_then(Value::as_str)
        .context("generated argument schema is missing x-postgres-type")?;
    ensure!(!raw.is_empty(), "generated PostgreSQL type is empty");
    ensure!(
        raw.chars().all(|character| {
            character.is_ascii_alphanumeric()
                || matches!(
                    character,
                    '_' | ' ' | '.' | ',' | '"' | '[' | ']' | '(' | ')'
                )
        }),
        "generated PostgreSQL type contains unsafe syntax: '{raw}'"
    );

    let schema_is_array = type_includes(schema, "array");
    let normalized = raw.trim_matches('"').to_ascii_lowercase();
    let pseudo_array = matches!(normalized.as_str(), "anyarray" | "anycompatiblearray");
    let pseudo_scalar = matches!(
        normalized.as_str(),
        "any"
            | "anyelement"
            | "anynonarray"
            | "anyenum"
            | "anycompatible"
            | "anycompatiblenonarray"
    );

    if pseudo_array {
        return Ok(("text[]".to_string(), true));
    }
    if pseudo_scalar {
        return Ok(("text".to_string(), false));
    }

    let is_variadic_array = schema_is_array && !raw.trim_end().ends_with("[]");
    let postgres_type = if is_variadic_array {
        format!("{raw}[]")
    } else {
        raw.to_string()
    };
    Ok((postgres_type, schema_is_array))
}

fn parameter_sort_key(name: &str) -> (u8, usize, &str) {
    let ordinal = name
        .strip_prefix("arg")
        .and_then(|suffix| suffix.parse::<usize>().ok());
    match ordinal {
        Some(ordinal) => (0, ordinal, name),
        None => (1, usize::MAX, name),
    }
}

fn routine_body_schema(endpoint: &EndpointRecord) -> anyhow::Result<&Map<String, Value>> {
    endpoint
        .input_schema
        .pointer("/requestBody/content/application~1json/schema/properties")
        .and_then(Value::as_object)
        .context("generated routine input schema has no request-body properties")
}

fn success_schema(endpoint: &EndpointRecord) -> Option<&Value> {
    endpoint
        .output_schema
        .pointer("/200/content/application~1json/schema")
}

fn build_relation_sql(schema: &str, relation: &str, args: &Value) -> anyhow::Result<String> {
    let limit = args
        .get("limit")
        .map(|value| value.as_u64().context("limit must be a positive integer"))
        .transpose()?
        .unwrap_or(DEFAULT_RELATION_LIMIT);
    let offset = args
        .get("offset")
        .map(|value| {
            value
                .as_u64()
                .context("offset must be a non-negative integer")
        })
        .transpose()?
        .unwrap_or(0);
    ensure!(
        (1..=MAX_RELATION_LIMIT).contains(&limit),
        "limit must be between 1 and {MAX_RELATION_LIMIT}"
    );

    Ok(format!(
        "SELECT COALESCE(jsonb_agg(to_jsonb(row_value)), '[]'::jsonb) \
         FROM (SELECT * FROM {}.{} LIMIT {limit} OFFSET {offset}) AS row_value",
        quote_identifier(schema),
        quote_identifier(relation)
    ))
}

fn build_routine_sql(
    endpoint: &EndpointRecord,
    schema: &str,
    routine: &str,
    args: &Value,
) -> anyhow::Result<(String, Vec<BoundText>)> {
    let properties = routine_body_schema(endpoint)?;
    let empty_body = Map::new();
    let body = args
        .get("body")
        .and_then(Value::as_object)
        .unwrap_or(&empty_body);
    ensure!(
        properties.is_empty() || args.get("body").is_some(),
        "routine arguments must be supplied in the 'body' object"
    );
    let mut definitions = properties.iter().collect::<Vec<_>>();
    definitions.sort_by_key(|(name, _)| parameter_sort_key(name));

    let mut expressions = Vec::new();
    let mut values = Vec::new();
    let mut skipped_positional = false;
    for (name, argument_schema) in definitions {
        let Some(value) = body.get(name) else {
            if name
                .strip_prefix("arg")
                .is_some_and(|suffix| suffix.parse::<usize>().is_ok())
            {
                skipped_positional = true;
            }
            continue;
        };
        ensure!(
            !skipped_positional,
            "cannot supply positional argument '{name}' after omitting an earlier defaulted argument"
        );

        let (postgres_type, is_array) = trusted_postgres_type(argument_schema)?;
        values.push(BoundText::from_json(value, is_array)?);
        let placeholder = values.len();
        let cast = if postgres_type.eq_ignore_ascii_case("bytea") {
            format!("decode(${placeholder}::text, 'base64')")
        } else {
            format!("${placeholder}::text::{postgres_type}")
        };
        let positional = name
            .strip_prefix("arg")
            .is_some_and(|suffix| suffix.parse::<usize>().is_ok());
        expressions.push(if positional {
            cast
        } else {
            format!("{} => {cast}", quote_identifier(name))
        });
    }

    let invocation = format!(
        "{}.{}({})",
        quote_identifier(schema),
        quote_identifier(routine),
        expressions.join(", ")
    );
    let returns_set = success_schema(endpoint)
        .and_then(|schema| schema.get("type"))
        .and_then(Value::as_str)
        == Some("array");
    let sql = if returns_set {
        format!(
            "SELECT COALESCE(jsonb_agg(to_jsonb(result)), '[]'::jsonb) \
             FROM (SELECT {invocation} AS result) AS routine_rows"
        )
    } else {
        format!("SELECT to_jsonb({invocation})")
    };
    Ok((sql, values))
}

fn text_params(values: &[BoundText]) -> Vec<(&(dyn ToSql + Sync), Type)> {
    values
        .iter()
        .map(|value| (&value.value as &(dyn ToSql + Sync), Type::TEXT))
        .collect()
}

async fn configure_transaction(
    transaction: &Transaction<'_>,
    timeout_ms: u64,
) -> anyhow::Result<()> {
    let timeout = timeout_ms.max(1).to_string();
    transaction
        .query_one(
            "SELECT set_config('statement_timeout', $1, true)",
            &[&timeout],
        )
        .await
        .context("failed to configure PostgreSQL statement_timeout")?;
    Ok(())
}

async fn configure_session(client: &Client, timeout_ms: u64) -> anyhow::Result<()> {
    let timeout = timeout_ms.max(1).to_string();
    client
        .query_one(
            "SELECT set_config('statement_timeout', $1, false)",
            &[&timeout],
        )
        .await
        .context("failed to configure PostgreSQL statement_timeout")?;
    Ok(())
}

async fn connect(config: &Config, credentials: &RequestCredentials) -> anyhow::Result<Client> {
    let mut postgres = PostgresConfig::from_str(&config.url)
        .context("invalid PostgreSQL connection URL; expected postgresql://host/database")?;
    postgres
        .user(&credentials.username)
        .password(&credentials.password)
        .application_name(env!("CARGO_PKG_NAME"))
        .connect_timeout(Duration::from_millis(config.timeout_ms.max(1)));

    let tls = match MakeRustlsConnect::with_native_certs() {
        Ok((connector, errors)) => {
            for error in errors {
                tracing::debug!(error = %error, "ignored unusable native TLS certificate");
            }
            connector
        }
        Err(errors) => {
            tracing::warn!(
                count = errors.len(),
                "native TLS roots could not be loaded; using webpki roots"
            );
            MakeRustlsConnect::with_webpki_roots()
        }
    };
    let (client, connection) = postgres
        .connect(tls)
        .await
        .map_err(|error| postgres_error("failed to connect to PostgreSQL", error))?;
    tokio::spawn(async move {
        if let Err(error) = connection.await {
            tracing::debug!(error = %error, "PostgreSQL connection task ended");
        }
    });
    Ok(client)
}

pub async fn test_connection(
    config: &Config,
    credentials: &RequestCredentials,
) -> anyhow::Result<Value> {
    let client = connect(config, credentials).await?;
    let row = client
        .query_one(
            "SELECT current_database(), current_user, current_setting('server_version')",
            &[],
        )
        .await
        .map_err(|error| postgres_error("PostgreSQL connection test failed", error))?;
    Ok(json!({
        "database": row.get::<_, String>(0),
        "user": row.get::<_, String>(1),
        "server_version": row.get::<_, String>(2),
    }))
}

pub async fn execute_operation(
    endpoint: &EndpointRecord,
    config: &Config,
    credentials: &RequestCredentials,
    args: &Value,
) -> anyhow::Result<Value> {
    let target = parse_endpoint_target(&endpoint.path)?;
    let (sql, values) = match target {
        EndpointTarget::Relation { schema, name } => {
            ensure!(endpoint.method.eq_ignore_ascii_case("GET"));
            (build_relation_sql(&schema, &name, args)?, Vec::new())
        }
        EndpointTarget::Routine { schema, name } => {
            ensure!(endpoint.method.eq_ignore_ascii_case("POST"));
            build_routine_sql(endpoint, &schema, &name, args)?
        }
    };

    let mut client = connect(config, credentials).await?;
    let transaction = client.build_transaction().read_only(true).start().await?;
    configure_transaction(&transaction, config.timeout_ms).await?;
    let params = text_params(&values);
    let row = transaction
        .query_typed_one(&sql, &params)
        .await
        .map_err(|error| {
            postgres_error(
                &format!("PostgreSQL operation '{}' failed", endpoint.operation_id),
                error,
            )
        })?;
    let Json(result) = row
        .try_get::<_, Json<Value>>(0)
        .context("PostgreSQL operation result could not be converted to JSON")?;
    transaction.commit().await?;
    Ok(result)
}

fn normalized_statement(sql: &str) -> anyhow::Result<&str> {
    let sql = sql.trim();
    ensure!(!sql.is_empty(), "sql must not be empty");
    let sql = sql.strip_suffix(';').unwrap_or(sql).trim_end();
    ensure!(!sql.is_empty(), "sql must not be empty");
    Ok(sql)
}

fn is_row_query(sql: &str) -> bool {
    let first = sql
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    matches!(first.as_str(), "SELECT" | "WITH" | "VALUES" | "TABLE")
}

pub async fn execute_parameterized_sql(
    config: &Config,
    credentials: &RequestCredentials,
    sql: &str,
    parameters: &[Value],
    max_rows: Option<usize>,
) -> anyhow::Result<Value> {
    let sql = normalized_statement(sql)?;
    let max_rows = max_rows.unwrap_or(DEFAULT_SQL_MAX_ROWS);
    ensure!(
        (1..=MAX_SQL_MAX_ROWS).contains(&max_rows),
        "max_rows must be between 1 and {MAX_SQL_MAX_ROWS}"
    );
    let values = parameters
        .iter()
        .map(|value| BoundText::from_json(value, false))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let params = text_params(&values);

    let client = connect(config, credentials).await?;
    configure_session(&client, config.timeout_ms).await?;
    // query_typed/execute_typed fix every parameter's wire type as TEXT.
    // PostgreSQL's extended protocol therefore never parses a value as SQL.
    // Callers use explicit casts such as `$1::text::integer` for non-text
    // values. Both APIs prepare exactly one statement, so stacked commands
    // are rejected by PostgreSQL before execution.
    if is_row_query(sql) {
        let fetch_rows = max_rows + 1;
        let wrapped =
            format!("SELECT to_jsonb(query_row) FROM ({sql}) AS query_row LIMIT {fetch_rows}");
        let mut rows = client
            .query_typed(&wrapped, &params)
            .await
            .map_err(|error| postgres_error("execute_sql query failed", error))?;
        let truncated = rows.len() > max_rows;
        rows.truncate(max_rows);
        let rows = rows
            .into_iter()
            .map(|row| row.try_get::<_, Json<Value>>(0).map(|Json(value)| value))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(json!({
            "rows": rows,
            "row_count": rows.len(),
            "truncated": truncated,
            "affected_rows": Value::Null,
        }))
    } else {
        let affected_rows = client
            .execute_typed(sql, &params)
            .await
            .map_err(|error| postgres_error("execute_sql statement failed", error))?;
        Ok(json!({
            "rows": [],
            "row_count": 0,
            "truncated": false,
            "affected_rows": affected_rows,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoint(path: &str, input_schema: Value, output_schema: Value) -> EndpointRecord {
        EndpointRecord {
            operation_id: "operation".to_string(),
            path: path.to_string(),
            method: if path.starts_with("/relations/") {
                "GET"
            } else {
                "POST"
            }
            .to_string(),
            summary: None,
            description: None,
            input_schema,
            output_schema,
        }
    }

    #[test]
    fn quotes_identifiers_and_rejects_non_catalog_paths() {
        assert_eq!(quote_identifier("odd\"name"), "\"odd\"\"name\"");
        assert_eq!(
            parse_endpoint_target("/relations/pg_catalog/pg_class").unwrap(),
            EndpointTarget::Relation {
                schema: "pg_catalog".to_string(),
                name: "pg_class".to_string()
            }
        );
        assert!(parse_endpoint_target("/arbitrary/sql").is_err());
    }

    #[test]
    fn relation_sql_uses_only_validated_numeric_pagination() {
        let sql = build_relation_sql(
            "pg_catalog",
            "pg_stat_activity",
            &json!({"limit": 25, "offset": 5}),
        )
        .unwrap();
        assert!(sql.contains("\"pg_catalog\".\"pg_stat_activity\""));
        assert!(sql.contains("LIMIT 25 OFFSET 5"));
        assert!(build_relation_sql("pg_catalog", "pg_class", &json!({"limit": 10001})).is_err());
    }

    #[test]
    fn routine_sql_binds_values_and_quotes_named_arguments() {
        let endpoint = endpoint(
            "/routines/pg_catalog/set_config/text_text_boolean_deadbeef",
            json!({
                "requestBody": {"content": {"application/json": {"schema": {
                    "properties": {
                        "setting_name": {"type": "string", "x-postgres-type": "text"},
                        "new_value": {"type": "string", "x-postgres-type": "text"},
                        "is_local": {"type": "boolean", "x-postgres-type": "boolean"}
                    }
                }}}}
            }),
            json!({"200": {"content": {"application/json": {"schema": {"type": "string"}}}}}),
        );
        let (sql, values) = build_routine_sql(
            &endpoint,
            "pg_catalog",
            "set_config",
            &json!({"body": {
                "setting_name": "application_name",
                "new_value": "value'); DROP TABLE users; --",
                "is_local": true
            }}),
        )
        .unwrap();
        assert!(
            sql.contains("\"setting_name\" => $3::text::text")
                || sql.contains("\"setting_name\" => $")
        );
        assert!(!sql.contains("DROP TABLE"));
        assert_eq!(values.len(), 3);
        assert!(values.iter().any(|value| {
            value
                .value
                .as_deref()
                .is_some_and(|value| value.contains("DROP TABLE"))
        }));
    }

    #[test]
    fn execute_sql_accepts_single_read_or_write_statement_shapes() {
        assert_eq!(normalized_statement(" SELECT 1; ").unwrap(), "SELECT 1");
        assert_eq!(
            normalized_statement("UPDATE users SET admin = true").unwrap(),
            "UPDATE users SET admin = true"
        );
        assert!(is_row_query(
            "WITH values AS (SELECT 1) SELECT * FROM values"
        ));
        assert!(!is_row_query("DELETE FROM users"));
        assert!(normalized_statement("; ").is_err());
    }

    #[test]
    fn arrays_are_encoded_as_postgresql_literals_without_sql_interpolation() {
        assert_eq!(
            postgres_array_literal(&json!(["a", "b\"c", null, [1, 2]])).unwrap(),
            "{\"a\",\"b\\\"c\",NULL,{\"1\",\"2\"}}"
        );
    }
}
