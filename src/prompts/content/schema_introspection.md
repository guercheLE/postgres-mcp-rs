# Guided workflow: schema introspection

This sub-workflow is designed to be run as an isolated sub-task where
possible -- if you were delegated here from `postgres`'s routing, or your
environment otherwise supports running this as its own sub-task, everything
you need is in this prompt's own text plus the parameters already listed
above; report back only a short summary when done rather than the full
step-by-step trace.

Every operation below is described as a capability to search for, never as a
specific `operationId` -- this server embeds catalogs for six PostgreSQL
versions (14 through 19beta2), and the exact operation and its response
schema can differ across them. Call `search` with the described capability,
`get` the operationId it resolves to, and read that operation's *current*
schema before relying on any field name in it -- never assume a column name
from memory or from a previous run.

## Step 0 -- gather what's known

Check the "Context already provided" header above first. `schema_name` and
`table_name` narrow every step below, but neither is required -- omit them to
browse broadly.

## Step 1 -- discover schemas and tables

Search for how to list schemas and tables (the SQL-standard
`information_schema.tables` view, or PostgreSQL's own `pg_tables`/`pg_class`
catalog). Filter to `schema_name` if given; otherwise list all
non-system schemas first (system schemas like `pg_catalog` and
`information_schema` are rarely what the user means by "my tables").

## Step 2 -- columns and types

Search for how to list a table's columns, data types, nullability, and
defaults (`information_schema.columns`). Confirm the table actually exists
in Step 1 before asking about its columns -- a typo'd `table_name` should
surface as "no such table" rather than an empty, misleading column list.

## Step 3 -- constraints

Primary keys, foreign keys, unique, and check constraints are split across
several catalog views rather than one: search for the operations covering
`information_schema.table_constraints` (constraint names/types),
`key_column_usage` (which columns each constraint covers),
`referential_constraints` (which foreign key points at which table), and
`check_constraints` (the actual check expression). Cross-reference them by
constraint name rather than assuming one view has everything.

## Step 4 -- indexes

Search for how to list a table's indexes (`pg_indexes` for the human-readable
`CREATE INDEX` text, `pg_index` for structured flags like uniqueness and the
indexed expression/columns). Note whether an index enforces a unique or
primary-key constraint versus existing purely for query performance --
they're not the same thing even when they cover the same columns.

## Step 5 -- views, materialized views, and sequences (as needed)

Only if the user's goal touches them: search for the operations covering
`pg_views`/`pg_matviews` (view/matview definitions -- note materialized views
need an explicit refresh and are not automatically kept in sync with their
underlying tables) and `pg_sequences` (current value, increment, and which
column owns the sequence, if any).

## Composing with other workflows

- Who can read/write what you just discovered overlaps with
  `postgres-roles-permissions`.
- Whether a query against these tables is using the indexes you found
  overlaps with `postgres-query-performance`.
- Row-count/bloat health for a specific table overlaps with
  `postgres-vacuum-maintenance`.
- Ad hoc queries against the schema you just mapped overlap with
  `postgres-data-profiling`.

Fetch those prompts by name for more detail rather than assuming their
content from this one.
