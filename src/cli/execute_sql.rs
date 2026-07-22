use postgres_mcp::auth::auth_manager::AuthManager;
use postgres_mcp::core::config_manager::load_config;
use postgres_mcp::core::config_schema::Transport;
use postgres_mcp::tools::execute_sql_tool::execute_sql;

pub async fn run(sql: &str, parameters_json: &str, max_rows: Option<usize>) -> anyhow::Result<()> {
    let config = load_config(serde_json::Map::new())?;
    let parameters: Vec<serde_json::Value> = serde_json::from_str(parameters_json)?;
    let mut auth_manager = AuthManager::new(config.auth_method);
    let credentials = auth_manager
        .postgres_credentials(Transport::Stdio, None)
        .await?;
    let result = execute_sql(&config, &credentials, sql, &parameters, max_rows).await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
