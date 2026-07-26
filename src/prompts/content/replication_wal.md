# Guided workflow: replication and WAL

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

Check the "Context already provided" header above first. `slot_name` and
`publication_name` narrow the steps below, but neither is required.

## Step 1 -- primary-side status

Search for how to list replication connections from standbys
(`pg_stat_replication`: each standby's state, sent/write/flush/replay LSNs)
and replication slots (`pg_replication_slots`, filtered to `slot_name` if
given: active/inactive, and how much WAL a slot is retaining). Also search
for current WAL activity stats (`pg_stat_wal`).

## Step 2 -- standby-side status

If connected to a standby, search for how to read its WAL receiver status
(`pg_stat_wal_receiver`) and its last-received/last-replayed WAL positions,
and whether WAL replay is currently paused.

## Step 3 -- computing lag correctly

Don't eyeball raw LSN values to estimate lag -- they're not simple numbers
to subtract by hand. Search for the LSN-diff operation
(`pg_wal_lsn_diff`) and call it with the two LSNs from Steps 1/2 (e.g.
current WAL position minus the standby's last-replayed position) to get an
actual byte distance.

## Step 4 -- logical replication (as needed)

Only if the user's goal is logical (not physical) replication: search for
publications (`pg_publication`, `pg_publication_tables` -- which tables a
publication actually includes) and subscriptions
(`pg_subscription`, `pg_stat_subscription` for a subscriber's own status).
Filter to `publication_name` if given.

## Step 5 -- pausing, resuming, or advancing (gated)

Pausing/resuming WAL replay on a standby, or manually advancing a
replication slot's position, changes what a standby will and won't apply --
treat these as destructive-ish operations:

1. Confirm intent with the user before calling; state plainly what will
   change (e.g. "this standby will stop applying new WAL until resumed").
2. After calling, re-read the relevant status operation from Step 1/2 (or
   the WAL-replay-pause-state operation) to confirm the state actually
   changed -- don't assume a non-error response means the change took
   effect.

## Composing with other workflows

- Base backups (a different way of getting a copy of the primary) overlap
  with `postgres-server-health-config`.
- FDW/foreign-server setups sometimes get confused with logical replication
  -- if the user's actual goal is querying another database rather than
  streaming changes to it, see `postgres-extensions-fdw` instead.

Fetch those prompts by name for more detail rather than assuming their
content from this one.
