# PostgreSQL catalog workflow menu

Match the user's goal to one of the guided sub-workflows below, then fetch
that prompt by name (`prompts/get`) for step-by-step instructions.

**Two live-execution paths -- pick the right one.** This server exposes
`search`/`get`/`call` for pre-validated PostgreSQL catalog operations
(`pg_catalog` and `information_schema` functions/views), plus a separate
`execute_sql` tool for exactly one parameterized raw SQL statement. Prefer
`search` -> `get` -> `call` whenever a catalog operation covers the need --
its input/output shape is already validated against a known schema. Reach for
`execute_sql` only when no catalog operation exists for the action: this
catalog has **no** operation for `EXPLAIN`, `VACUUM`, `ANALYZE`,
`GRANT`/`REVOKE`, or any DDL (`CREATE`/`ALTER`/`DROP`) -- these are SQL
commands, not catalog functions, and every sub-workflow below calls this out
explicitly wherever it applies.

**Delegate whole sub-workflows when you can.** If your environment provides a
way to run a sub-task/agent in an isolated context, delegate the entire
matched sub-workflow to it: hand the sub-task the sub-workflow's prompt name
and whatever parameters you already know, let it fetch that prompt itself and
carry out every step -- including its own `search`/`get`/`call`/`execute_sql`
traffic -- entirely in its own context, and have it report back only a short
summary (what was found/confirmed, and anything still needed from the user).
Only run a sub-workflow's steps directly in this conversation if no such
delegation mechanism is available.

## Sub-workflows

- **`postgres-schema-introspection`** -- tables, columns, types,
  constraints, indexes, views, and sequences across schemas.
- **`postgres-roles-permissions`** -- roles, role membership, object
  grants, and row-level security policies.
- **`postgres-session-locks`** -- active sessions, blocking chains,
  locks, and safely cancelling or terminating a backend.
- **`postgres-replication-wal`** -- replication status, replication
  slots, publications/subscriptions, and WAL position/lag.
- **`postgres-vacuum-maintenance`** -- autovacuum/analyze activity,
  dead-tuple/bloat signals, and running VACUUM/ANALYZE manually.
- **`postgres-query-performance`** -- diagnosing a slow query: is it
  running now, its EXPLAIN plan, and index usage.
- **`postgres-server-health-config`** -- live configuration
  (`pg_settings`), background writer/checkpointer/archiver stats, and
  physical backups.
- **`postgres-extensions-fdw`** -- installed/available extensions and
  foreign data wrapper objects.
- **`postgres-data-profiling`** -- ad hoc data exploration,
  aggregation, and row-level querying via `execute_sql`.

If the user's goal doesn't clearly match a sub-workflow above, ask a short
clarifying question rather than guessing which one they mean.
