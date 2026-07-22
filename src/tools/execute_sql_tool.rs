//! Injection-safe generic PostgreSQL execution.

use serde_json::Value;

use crate::auth::request_credentials::RequestCredentials;
use crate::core::config_schema::Config;
use crate::services::postgres_client::execute_parameterized_sql;

pub async fn execute_sql(
    config: &Config,
    credentials: &RequestCredentials,
    sql: &str,
    parameters: &[Value],
    max_rows: Option<usize>,
) -> anyhow::Result<Value> {
    execute_parameterized_sql(config, credentials, sql, parameters, max_rows).await
}
