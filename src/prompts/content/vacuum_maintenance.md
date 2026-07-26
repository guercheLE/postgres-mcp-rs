# Guided workflow: vacuum and maintenance

This sub-workflow is designed to be run as an isolated sub-task where
possible -- if you were delegated here from `postgres`'s routing, or your
environment otherwise supports running this as its own sub-task, everything
you need is in this prompt's own text plus the parameters already listed
above; report back only a short summary when done rather than the full
step-by-step trace.

Every operation below is described as a capability to search for, never as a
specific `operationId` -- the exact operation and its response schema can
differ across the six PostgreSQL versions this server supports. Call
`search` with the described capability, `get` the operationId it resolves
to, and read that operation's *current* schema before relying on any field
name in it.

## Step 0 -- gather what's known

Check the "Context already provided" header above first. `schema_name` and
`table_name` narrow every step below, but neither is required -- omit them
to survey every table.

## Step 1 -- per-table health signals

Search for how to read per-table statistics (`pg_stat_user_tables`, or
`pg_stat_sys_tables` for system tables): live/dead tuple counts, and the
timestamps of the last manual and automatic vacuum/analyze. A high
`n_dead_tup` relative to `n_live_tup`, or a `last_autovacuum` far in the
past, both point at bloat or a starved autovacuum -- don't conclude a table
is "unmaintained" from one signal alone.

## Step 2 -- in-flight activity

Search for how to check whether a vacuum or analyze is currently running on
a table right now (`pg_stat_progress_vacuum`, `pg_stat_progress_analyze`).
If one is already in progress, don't kick off another -- point the user at
its current progress instead.

## Step 3 -- running VACUUM/ANALYZE manually (gated, execute_sql)

**No catalog operation exists for `VACUUM`/`ANALYZE`** -- they're SQL
commands, not catalog functions. Run them via `execute_sql` with the literal
statement text, e.g. `VACUUM (ANALYZE, VERBOSE) schema.table`. Table and
schema names can't be bound as `$1`-style parameters -- parameters are for
values only -- so validate the identifier against Step 1's own result (or
`postgres-schema-introspection`) before including it in the SQL text, rather
than trusting user-supplied spelling verbatim.

Gate: don't report the vacuum/analyze as done until a follow-up read of Step
1's stats shows `last_vacuum`/`last_analyze` updated and `n_dead_tup`
reduced. An empty command-complete result from `execute_sql` confirms the
statement ran, not that it changed anything meaningful (e.g. a table with no
dead tuples completes instantly and changes nothing).

## Composing with other workflows

- Confirming exactly which table/schema is involved overlaps with
  `postgres-schema-introspection`.
- A vacuum currently holding a lock that's blocking other queries overlaps
  with `postgres-session-locks`.
- Whether stale statistics (rather than bloat) explain a bad query plan
  overlaps with `postgres-query-performance`.

Fetch those prompts by name for more detail rather than assuming their
content from this one.
