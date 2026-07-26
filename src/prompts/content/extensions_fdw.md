# Guided workflow: extensions and foreign data wrappers

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

Check the "Context already provided" header above first. `extension_name`
and `server_name` narrow the steps below, but neither is required.

## Step 1 -- installed vs. available extensions

Search for how to list extensions already installed in the current database
(`pg_extension`) versus extensions available to install but not yet enabled
(`pg_available_extensions`, `pg_available_extension_versions` for which
specific versions are available). Don't conflate the two -- "is X
available" and "is X installed" are different questions with different
catalog operations.

## Step 2 -- foreign data wrapper objects

If the user's goal is querying another database/data source rather than
installing a general-purpose extension, search for the FDW object family:
foreign data wrappers, foreign servers (`server_name` narrows here), foreign
tables, and user mappings (which local role maps to which credentials on the
remote side). These exist in both `information_schema` (portable, SQL-
standard shape) and `pg_catalog` (PostgreSQL-native, more detail) -- either
covers the same objects.

## Step 3 -- installing or removing an extension (gated, execute_sql)

**No catalog operation exists for `CREATE EXTENSION`/`DROP EXTENSION`** --
they're SQL commands, not catalog functions. Run them via `execute_sql`.
Installing an extension is a schema-level change (it can add tables, types,
functions) -- confirm with the user which extension and version before
running it, and prefer specifying the version explicitly (from Step 1's
available-versions list) rather than letting it default silently.

Gate: after installing/removing, re-read `pg_extension` (Step 1) to confirm
the change actually took effect before declaring success.

## Composing with other workflows

- If the real goal turns out to be streaming changes between databases
  rather than querying across them, see `postgres-replication-wal` instead
  -- FDW and logical replication solve related but different problems and
  are easy to conflate.
- Once a foreign table exists, querying it behaves like any other table --
  see `postgres-data-profiling`.

Fetch those prompts by name for more detail rather than assuming their
content from this one.
