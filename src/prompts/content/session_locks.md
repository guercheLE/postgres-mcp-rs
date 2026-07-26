# Guided workflow: sessions and locks

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

Check the "Context already provided" header above first. `pid` and
`database_name` narrow the steps below, but neither is required -- omit
them to see every active session.

## Step 1 -- what's running right now

Search for how to list active sessions/connections (`pg_stat_activity`):
their pid, database, user, current query text, and state (active, idle,
idle in transaction, etc). Filter to `database_name`/`pid` if given.

## Step 2 -- lock waits and blocking chains

Search for how to list current locks (`pg_locks`) and, separately, how to
find which pid(s) are blocking a given pid (`pg_blocking_pids`). A session
sitting in `active` state with no progress is very often waiting on a lock
held by another session -- check this before assuming a query is simply
slow. Trace the blocking chain by following the blocker's own pid back
through Step 1/2 if it, in turn, appears blocked.

## Step 3 -- cancelling or terminating a session (gated)

Two distinct operations exist, and they are not interchangeable:

- **Cancel** the session's current query (comparable to Ctrl-C) -- the
  connection stays open, only the in-flight statement is aborted.
- **Terminate** the backend entirely -- the connection itself is dropped,
  aborting any open transaction.

Search for and use the cancel operation unless the user specifically wants
the connection itself closed. Before calling either:

1. Confirm the target `pid` and the query text/session details from Step 1
   with the user -- never cancel/terminate based on a guessed or
   previous-run pid.
2. After calling, re-read `pg_stat_activity` (Step 1) to confirm the
   session is actually gone (terminate) or idle again (cancel). A
   non-error response from the call alone is not sufficient confirmation --
   the target may already have disconnected or the signal may not have been
   delivered yet.

## Step 4 -- advisory locks (a separate mechanism)

If the user's question is about an application-level lock rather than a
row/table lock, search for the advisory lock operations (session-level and
transaction-level acquire/release, shared and exclusive variants). These are
managed entirely by application code calling them explicitly -- they don't
appear in `pg_locks` the same way row/table locks do unless specifically
queried for lock type `advisory`.

## Composing with other workflows

- "Why is this specific query slow" once it's confirmed *not* to be lock-
  blocked overlaps with `postgres-query-performance`.
- Autovacuum holding locks on a table overlaps with
  `postgres-vacuum-maintenance`.

Fetch those prompts by name for more detail rather than assuming their
content from this one.
