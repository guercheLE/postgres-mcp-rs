# Guided workflow: roles and permissions

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

Check the "Context already provided" header above first. `role_name`,
`schema_name`, and `table_name` narrow the questions below, but none are
required -- omit them to browse broadly.

## Step 1 -- roles and membership

Search for how to list roles and their attributes (login capability,
superuser, ability to create roles/databases) and role membership (which
roles are members of which other roles, i.e. inherited privileges).

**Sensitivity gate:** the underlying catalog also exposes password hashes to
a superuser. Never surface a password hash in a response, even if the
connected role happens to be able to read it -- the user asking "what are
this role's attributes" almost never means "show me its hash."

## Step 2 -- object-level grants

Search for how to list grants on tables, columns, routines, sequences, and
user-defined types, scoped to `schema_name`/`table_name`/`role_name` as
given. These are split by object type across several
`information_schema.*_privileges` views -- check the one matching the
object type the user actually cares about rather than assuming table grants
cover everything.

## Step 3 -- row-level security (a separate layer)

Search for how to list row-level security policies (`pg_policies`). RLS
policies are an *additional* filter applied on top of whatever GRANT already
allows -- a role can have full SELECT/UPDATE grants on a table and still see
or modify no rows (or a restricted subset) if RLS is enabled and no policy
matches it. Don't report "this role has full access" from grants alone
without checking whether RLS is enabled on the table.

## Step 4 -- granting or revoking (gated, execute_sql)

**No catalog operation exists for `GRANT`/`REVOKE`** -- they're SQL commands,
not catalog functions. Confirm the exact role, privilege, and object with the
user, then run the statement via `execute_sql`. Object and role identifiers
can't be bound as `$1`-style parameters (parameters are for values only) --
validate the identifier against what Step 1/2 actually returned before
including it in the SQL text, rather than trusting user-supplied spelling
verbatim.

Gate: don't report the grant/revoke as done until a follow-up read of the
relevant `*_privileges` view (Step 2) confirms the change actually took
effect. A non-error response from `execute_sql` alone is not sufficient
confirmation.

## Composing with other workflows

- Finding the tables/schemas a grant question is actually about overlaps
  with `postgres-schema-introspection`.
- "What is this role doing right now" (as opposed to what it's *allowed* to
  do) overlaps with `postgres-session-locks`.

Fetch those prompts by name for more detail rather than assuming their
content from this one.
