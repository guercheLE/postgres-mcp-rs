// PostgreSQL 18.4 catalog MCP server — adapted for PostgreSQL native protocol.

use postgres_mcp::auth::auth_manager::AuthManager;
use postgres_mcp::core::config_manager::load_config;
use postgres_mcp::core::config_schema::Transport;
use postgres_mcp::services::postgres_client::test_connection;

pub async fn run() -> anyhow::Result<()> {
    let config = load_config(serde_json::Map::new())?;
    let mut auth_manager = AuthManager::new(config.auth_method);
    let credentials = auth_manager
        .postgres_credentials(Transport::Stdio, None)
        .await?;
    let details = test_connection(&config, &credentials).await?;
    println!("connection OK: {}", serde_json::to_string(&details)?);
    Ok(())
}
