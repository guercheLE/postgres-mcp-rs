# Guided workflow: query performance

This sub-workflow is designed to be run as an isolated sub-task where
possible -- if you were delegated here from `postgres`'s routing, or your
environment otherwise supports running this as its own sub-task, everything
you need is in this prompt's own text plus the parameters already listed
above; report back only a short summary when done rather than the full
step-by-step trace.

Every catalog operation below is described as a capability to search for,
never as a specific `operationId` -- the exact operation and its response
schema can differ across the six PostgreSQL versions this server supports.
Call `search` with the described capability, `get` the operationId it
resolves to, and read that operation's *current* schema before relying on
any field name in it.

## Step 0 -- gather what's known

Check the "Context already provided" header above first. `query_text` and
`table_name` narrow the steps below, but neither is required.

## Step 1 -- is it running right now?

Before reasoning about a plan, check whether the query is actually executing
this moment and, if so, whether it's blocked rather than slow. Fetch the
`postgres-session-locks` prompt and follow its Step 1/2 (`pg_stat_activity`,
`pg_locks`, `pg_blocking_pids`) -- a query stuck on a lock wait needs a very
different fix than a query that's genuinely doing a lot of work.

## Step 2 -- get an EXPLAIN plan (gated, execute_sql)

**No catalog operation exists for `EXPLAIN`** -- it's a SQL command, not a
catalog function. Run it via `execute_sql`:
`EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON) <query>`.

**Caution:** `ANALYZE` in an `EXPLAIN` statement actually executes the
query, including any side effects. For anything that isn't a plain `SELECT`
(an `UPDATE`/`INSERT`/`DELETE`, or a `SELECT` calling a side-effecting
function), either wrap it in a transaction you roll back afterward, or drop
`ANALYZE` and use a plain `EXPLAIN` (estimated costs only, no execution) --
confirm which the user actually wants before running it.

Read the plan for: sequential scans on large tables where an index scan
would be expected, large row-estimate-vs-actual mismatches (a sign of stale
statistics), and where most of the actual time is spent (nested loops over
large row counts are a common culprit).

## Step 3 -- cross-check index usage

Search for how to list a table's indexes and their usage counts
(`pg_stat_user_indexes`'s `idx_scan`, and `pg_indexes` for the actual
`CREATE INDEX` definitions). An index that exists but has `idx_scan` near
zero either isn't a good match for this query's predicates or isn't being
chosen by the planner for another reason (e.g. stale statistics, or the
planner correctly judging a sequential scan cheaper on a small table).

## Step 4 -- aggregate query history (if available)

This server's embedded catalog does **not** include `pg_stat_statements` --
it's a contrib extension, not a built-in catalog view, so it never appears
in `search` results. If the user wants aggregate stats across many
executions (not just this one plan) rather than a one-off EXPLAIN, ask
whether `pg_stat_statements` is already installed on the target database; if
so, query its view directly via `execute_sql` (it behaves like any other
table/view once installed). If it isn't installed, installing it is a
`postgres-extensions-fdw` matter (extension install is DDL, gated there).

## Composing with other workflows

- Stale statistics from a starved autovacuum overlap with
  `postgres-vacuum-maintenance`.
- Confirming exact table/column/constraint shape overlaps with
  `postgres-schema-introspection`.
- Lock waits and blocking sessions overlap with `postgres-session-locks`.

Fetch those prompts by name for more detail rather than assuming their
content from this one.
