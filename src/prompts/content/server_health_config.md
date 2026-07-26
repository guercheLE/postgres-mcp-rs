# Guided workflow: server health and configuration

This sub-workflow is designed to be run as an isolated sub-task where
possible -- if you were delegated here from `postgres`'s routing, or your
environment otherwise supports running this as its own sub-task, everything
you need is in this prompt's own text plus the parameters already listed
above; report back only a short summary when done rather than the full
step-by-step trace.

Every operation below is described as a capability to search for, never as a
specific `operationId` -- the exact operation and its response schema can
differ across the six PostgreSQL versions this server supports, and some
catalog views (e.g. the checkpointer's own stats view) were split out of an
older, combined view in more recent PostgreSQL releases. Call `search` with
the described capability, `get` the operationId it resolves to, and read
that operation's *current* schema before relying on any field name in it.

## Step 0 -- gather what's known

Check the "Context already provided" header above first. `setting_name`
narrows Step 1, but isn't required -- omit it to browse all settings.

## Step 1 -- live configuration

Search for how to read current configuration settings (`pg_settings`),
filtered to `setting_name` if given. Pay attention to two columns beyond the
value itself: `context` (whether changing this setting needs just a config
reload, a full server restart, or can be set per-session) and
`pending_restart` (a value was changed in the config file but hasn't taken
effect yet because the required restart/reload hasn't happened).

## Step 2 -- why a config edit didn't take effect

If the user edited `postgresql.conf`/`pg_hba.conf` directly and the change
doesn't seem to be applied, search for the file-level views
(`pg_file_settings` for `postgresql.conf`, `pg_hba_file_rules` for
`pg_hba.conf`, `pg_ident_file_mappings` for user name mapping) -- they
surface parse errors and which file/line actually won for a given setting,
which is often the actual cause rather than a missing restart.

## Step 3 -- background process throughput

Search for how to read background writer, checkpointer, and archiver
statistics (`pg_stat_bgwriter`, `pg_stat_checkpointer`, `pg_stat_archiver`).
Frequent forced checkpoints or a high archiver failure count are both
signals worth surfacing even if the user only asked a general "is the server
healthy" question.

## Step 4 -- physical backups (gated)

Search for the online-backup bracket operations (backup start, backup
stop). These bracket a single backup: calling start begins an exclusive or
non-exclusive backup mode, and the backup isn't valid/complete until stop is
called. Before calling start:

1. Confirm the user has a clear plan to call stop afterward (or that their
   backup tooling does so automatically).
2. Check nothing else has already started a backup that's still in
   progress -- don't stack a second one on top.

## Composing with other workflows

- Checkpoint-triggered I/O spikes coinciding with vacuum activity overlap
  with `postgres-vacuum-maintenance`.
- Physical backups are a different mechanism from logical replication --
  see `postgres-replication-wal` if the user's actual goal is streaming
  changes rather than a point-in-time copy.

Fetch those prompts by name for more detail rather than assuming their
content from this one.
