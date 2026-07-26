//! `#[prompt_router]`-decorated `impl McpifyServer` block. Kept separate
//! from `core::mcp_server`'s `#[tool_router]` block -- see
//! `docs/mcp-prompts-workflow-plan.md`.

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{PromptMessage, Role};
use rmcp::{prompt, prompt_router};

use crate::core::mcp_server::McpifyServer;
use crate::prompts::{
    DataProfilingWorkflowArgs, ExtensionsFdwWorkflowArgs, MasterWorkflowArgs,
    QueryPerformanceWorkflowArgs, ReplicationWalWorkflowArgs, RolesPermissionsWorkflowArgs,
    SchemaIntrospectionWorkflowArgs, ServerHealthConfigWorkflowArgs, SessionLocksWorkflowArgs,
    VacuumMaintenanceWorkflowArgs, render_context_header,
};

#[prompt_router(vis = "pub")]
impl McpifyServer {
    #[prompt(
        name = "postgres",
        description = "Start here. Presents the available PostgreSQL catalog-introspection \
                        and diagnostic workflows, routes to the right guided sub-workflow \
                        based on the user's goal, and -- where the environment supports it -- \
                        delegates that whole sub-workflow to an isolated sub-task to spare \
                        this conversation's context window."
    )]
    async fn postgres_workflow_prompt(
        &self,
        Parameters(args): Parameters<MasterWorkflowArgs>,
    ) -> Vec<PromptMessage> {
        let header = render_context_header(&[("goal", args.goal.as_deref())]);
        vec![PromptMessage::new_text(
            Role::User,
            format!("{header}\n{}", include_str!("content/master.md")),
        )]
    }

    #[prompt(
        name = "postgres-schema-introspection",
        description = "Tables, columns, constraints, indexes, views, materialized views, and \
                        sequences across schemas -- discovering shape rather than guessing at \
                        column names."
    )]
    async fn postgres_workflow_schema_introspection_prompt(
        &self,
        Parameters(args): Parameters<SchemaIntrospectionWorkflowArgs>,
    ) -> Vec<PromptMessage> {
        let header = render_context_header(&[
            ("schema_name", args.schema_name.as_deref()),
            ("table_name", args.table_name.as_deref()),
        ]);
        vec![PromptMessage::new_text(
            Role::User,
            format!(
                "{header}\n{}",
                include_str!("content/schema_introspection.md")
            ),
        )]
    }

    #[prompt(
        name = "postgres-roles-permissions",
        description = "Roles, role membership, object-level grants (tables, columns, \
                        routines, sequences, types), and row-level security policies -- \
                        including the gated GRANT/REVOKE path via execute_sql."
    )]
    async fn postgres_workflow_roles_permissions_prompt(
        &self,
        Parameters(args): Parameters<RolesPermissionsWorkflowArgs>,
    ) -> Vec<PromptMessage> {
        let header = render_context_header(&[
            ("role_name", args.role_name.as_deref()),
            ("schema_name", args.schema_name.as_deref()),
            ("table_name", args.table_name.as_deref()),
        ]);
        vec![PromptMessage::new_text(
            Role::User,
            format!("{header}\n{}", include_str!("content/roles_permissions.md")),
        )]
    }

    #[prompt(
        name = "postgres-session-locks",
        description = "Active sessions/connections, blocking chains, locks, and safely \
                        cancelling or terminating a backend."
    )]
    async fn postgres_workflow_session_locks_prompt(
        &self,
        Parameters(args): Parameters<SessionLocksWorkflowArgs>,
    ) -> Vec<PromptMessage> {
        let header = render_context_header(&[
            ("pid", args.pid.as_deref()),
            ("database_name", args.database_name.as_deref()),
        ]);
        vec![PromptMessage::new_text(
            Role::User,
            format!("{header}\n{}", include_str!("content/session_locks.md")),
        )]
    }

    #[prompt(
        name = "postgres-replication-wal",
        description = "Physical/logical replication status, replication slots, \
                        publications/subscriptions, and WAL position/lag."
    )]
    async fn postgres_workflow_replication_wal_prompt(
        &self,
        Parameters(args): Parameters<ReplicationWalWorkflowArgs>,
    ) -> Vec<PromptMessage> {
        let header = render_context_header(&[
            ("slot_name", args.slot_name.as_deref()),
            ("publication_name", args.publication_name.as_deref()),
        ]);
        vec![PromptMessage::new_text(
            Role::User,
            format!("{header}\n{}", include_str!("content/replication_wal.md")),
        )]
    }

    #[prompt(
        name = "postgres-vacuum-maintenance",
        description = "Autovacuum/analyze activity, dead-tuple/bloat signals, and running \
                        VACUUM/ANALYZE manually via the gated execute_sql path."
    )]
    async fn postgres_workflow_vacuum_maintenance_prompt(
        &self,
        Parameters(args): Parameters<VacuumMaintenanceWorkflowArgs>,
    ) -> Vec<PromptMessage> {
        let header = render_context_header(&[
            ("schema_name", args.schema_name.as_deref()),
            ("table_name", args.table_name.as_deref()),
        ]);
        vec![PromptMessage::new_text(
            Role::User,
            format!(
                "{header}\n{}",
                include_str!("content/vacuum_maintenance.md")
            ),
        )]
    }

    #[prompt(
        name = "postgres-query-performance",
        description = "Diagnosing a slow query: is it running now, its EXPLAIN plan via \
                        execute_sql, and index usage -- rather than guessing at the cause."
    )]
    async fn postgres_workflow_query_performance_prompt(
        &self,
        Parameters(args): Parameters<QueryPerformanceWorkflowArgs>,
    ) -> Vec<PromptMessage> {
        let header = render_context_header(&[
            ("query_text", args.query_text.as_deref()),
            ("table_name", args.table_name.as_deref()),
        ]);
        vec![PromptMessage::new_text(
            Role::User,
            format!("{header}\n{}", include_str!("content/query_performance.md")),
        )]
    }

    #[prompt(
        name = "postgres-server-health-config",
        description = "Live configuration (pg_settings), why a config-file edit didn't take \
                        effect, background writer/checkpointer/archiver stats, and physical \
                        backups."
    )]
    async fn postgres_workflow_server_health_config_prompt(
        &self,
        Parameters(args): Parameters<ServerHealthConfigWorkflowArgs>,
    ) -> Vec<PromptMessage> {
        let header = render_context_header(&[("setting_name", args.setting_name.as_deref())]);
        vec![PromptMessage::new_text(
            Role::User,
            format!(
                "{header}\n{}",
                include_str!("content/server_health_config.md")
            ),
        )]
    }

    #[prompt(
        name = "postgres-extensions-fdw",
        description = "Installed vs. available extensions, and foreign data wrapper objects \
                        (foreign servers, foreign tables, user mappings)."
    )]
    async fn postgres_workflow_extensions_fdw_prompt(
        &self,
        Parameters(args): Parameters<ExtensionsFdwWorkflowArgs>,
    ) -> Vec<PromptMessage> {
        let header = render_context_header(&[
            ("extension_name", args.extension_name.as_deref()),
            ("server_name", args.server_name.as_deref()),
        ]);
        vec![PromptMessage::new_text(
            Role::User,
            format!("{header}\n{}", include_str!("content/extensions_fdw.md")),
        )]
    }

    #[prompt(
        name = "postgres-data-profiling",
        description = "Ad hoc data exploration, aggregation, and row-level querying via \
                        execute_sql, safely -- identifier validation, parameter binding, and \
                        result-size limits."
    )]
    async fn postgres_workflow_data_profiling_prompt(
        &self,
        Parameters(args): Parameters<DataProfilingWorkflowArgs>,
    ) -> Vec<PromptMessage> {
        let header = render_context_header(&[("table_name", args.table_name.as_deref())]);
        vec![PromptMessage::new_text(
            Role::User,
            format!("{header}\n{}", include_str!("content/data_profiling.md")),
        )]
    }
}
