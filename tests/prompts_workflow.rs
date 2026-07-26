//! Protocol-level `prompts/list`/`prompts/get` tests, kept out of
//! `src/core/mcp_server.rs`'s tool-focused `#[cfg(test)] mod tests` --
//! mirrors that module's duplex-transport pattern. See
//! `docs/mcp-prompts-workflow-plan.md`.

use std::sync::Arc;

use postgres_mcp::auth::auth_manager::AuthManager;
use postgres_mcp::core::config_schema::{AuthMethod, Config};
use postgres_mcp::core::mcp_server::McpifyServer;
use rmcp::ServiceExt;
use rmcp::model::GetPromptRequestParams;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Default)]
struct TestClient;

impl rmcp::ClientHandler for TestClient {}

fn server() -> McpifyServer {
    let config: Config = serde_json::from_value(serde_json::json!({
        "url": "postgresql://db.example/test",
        "auth_method": "password"
    }))
    .unwrap();
    McpifyServer::new(
        "18".to_string(),
        config,
        Arc::new(Mutex::new(AuthManager::new(AuthMethod::Password))),
    )
}

fn prompt_text(result: &rmcp::model::GetPromptResult) -> &str {
    &result.messages[0]
        .content
        .as_text()
        .expect("prompt message should be text content")
        .text
}

#[tokio::test]
async fn server_info_advertises_the_prompts_capability() {
    use rmcp::ServerHandler;
    let info = server().get_info();
    assert!(info.capabilities.prompts.is_some());
}

#[tokio::test]
async fn prompts_list_and_get_round_trip_over_the_mcp_protocol() {
    let (server_transport, client_transport) = tokio::io::duplex(64 * 1024);
    let server_task = tokio::spawn(async move {
        server().serve(server_transport).await?.waiting().await?;
        anyhow::Ok(())
    });
    let client = TestClient.serve(client_transport).await.unwrap();

    let prompts = client.list_all_prompts().await.unwrap();
    let names: Vec<&str> = prompts.iter().map(|p| p.name.as_ref()).collect();
    assert_eq!(
        names.len(),
        10,
        "expected exactly 10 prompts, got {names:?}"
    );
    for expected in [
        "postgres",
        "postgres-schema-introspection",
        "postgres-roles-permissions",
        "postgres-session-locks",
        "postgres-replication-wal",
        "postgres-vacuum-maintenance",
        "postgres-query-performance",
        "postgres-server-health-config",
        "postgres-extensions-fdw",
        "postgres-data-profiling",
    ] {
        assert!(names.contains(&expected), "missing prompt {expected}");
    }
    assert!(names.iter().all(|name| name.starts_with("postgres")));

    let schema_prompt = prompts
        .iter()
        .find(|p| p.name == "postgres-schema-introspection")
        .expect("postgres-schema-introspection should be advertised");
    let schema_args = schema_prompt
        .arguments
        .as_ref()
        .expect("postgres-schema-introspection should advertise arguments");
    for expected in ["schema_name", "table_name"] {
        assert!(
            schema_args.iter().any(|a| a.name == expected),
            "missing argument {expected}"
        );
    }
    assert!(
        schema_args.iter().all(|a| a.required == Some(false)),
        "every postgres-schema-introspection argument should be optional"
    );

    let roles_prompt = prompts
        .iter()
        .find(|p| p.name == "postgres-roles-permissions")
        .expect("postgres-roles-permissions should be advertised");
    let arg_names: Vec<&str> = roles_prompt
        .arguments
        .as_ref()
        .expect("postgres-roles-permissions should advertise arguments")
        .iter()
        .map(|a| a.name.as_str())
        .collect();
    for expected in ["role_name", "schema_name", "table_name"] {
        assert!(arg_names.contains(&expected), "missing argument {expected}");
    }
    assert!(
        roles_prompt
            .arguments
            .as_ref()
            .unwrap()
            .iter()
            .all(|a| a.required == Some(false)),
        "every postgres-roles-permissions argument should be optional"
    );

    // `postgres` with no arguments should link to every sub-workflow.
    let master = client
        .get_prompt(GetPromptRequestParams::new("postgres"))
        .await
        .unwrap();
    let master_text = prompt_text(&master);
    for expected in [
        "postgres-schema-introspection",
        "postgres-roles-permissions",
        "postgres-session-locks",
        "postgres-replication-wal",
        "postgres-vacuum-maintenance",
        "postgres-query-performance",
        "postgres-server-health-config",
        "postgres-extensions-fdw",
        "postgres-data-profiling",
    ] {
        assert!(
            master_text.contains(expected),
            "master prompt should mention {expected}"
        );
    }

    // `postgres-schema-introspection` with partial arguments should echo
    // the supplied values and list the still-missing ones.
    let mut partial_args = serde_json::Map::new();
    partial_args.insert("schema_name".to_string(), serde_json::json!("public"));
    let schema = client
        .get_prompt(
            GetPromptRequestParams::new("postgres-schema-introspection")
                .with_arguments(partial_args),
        )
        .await
        .unwrap();
    let schema_text = prompt_text(&schema);
    assert!(schema_text.contains("schema_name: public"));
    assert!(schema_text.contains("- table_name"));

    for prompt_name in [
        "postgres-roles-permissions",
        "postgres-session-locks",
        "postgres-replication-wal",
        "postgres-vacuum-maintenance",
        "postgres-query-performance",
        "postgres-server-health-config",
        "postgres-extensions-fdw",
        "postgres-data-profiling",
    ] {
        let prompt_res = client
            .get_prompt(GetPromptRequestParams::new(prompt_name))
            .await
            .unwrap();
        assert!(!prompt_text(&prompt_res).is_empty());
    }

    drop(client);
    tokio::time::timeout(std::time::Duration::from_secs(2), server_task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}
