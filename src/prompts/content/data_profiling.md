# Guided workflow: data profiling

This sub-workflow is designed to be run as an isolated sub-task where
possible -- if you were delegated here from `postgres`'s routing, or your
environment otherwise supports running this as its own sub-task, everything
you need is in this prompt's own text plus the parameters already listed
above; report back only a short summary when done rather than the full
step-by-step trace.

Unlike the other sub-workflows, this one is centered entirely on
`execute_sql` rather than `search`/`get`/`call` -- ad hoc row-level queries,
joins across application tables, and aggregations aren't catalog operations
at all.

## Step 0 -- gather what's known

Check the "Context already provided" header above first. `table_name`
narrows the steps below, but isn't required -- ask what the user actually
wants to learn about the data if it's unclear.

## Step 0.5 -- decide: catalog operation or execute_sql?

Before writing a query, check whether a catalog operation already answers
the question -- it's pre-validated against a known schema and cheaper to get
right. Reach for `execute_sql` specifically for: ad hoc joins/aggregations
across application tables, row-level filtering beyond what a catalog
operation's own parameters expose, or any SQL PostgreSQL only exposes as a
command (`EXPLAIN`, `VACUUM`, `ANALYZE`, `GRANT`/`REVOKE`, DDL -- see the
other sub-workflows for those specific gates).

## Step 1 -- validate identifiers before use

`execute_sql`'s `$1`/`$2`-style parameters bind *values* only -- table and
column names can never be parameterized that way. Before interpolating a
user-supplied table or column name into SQL text, validate it against
`information_schema.columns`/`tables` (fetch `postgres-schema-introspection`
for how) rather than trusting the spelling verbatim -- this is the
difference between a safe, parameterized query and one vulnerable to SQL
injection through an identifier.

## Step 2 -- bind every value as a parameter

Every caller-controlled *value* (a filter, a literal being compared, a limit
count that came from user input) must be bound as `$1`, `$2`, etc., never
string-concatenated into the SQL text -- this is what makes `execute_sql`
injection-safe. Cast non-text values explicitly where needed (parameters
arrive as text on the wire, e.g. `$1::text::integer`).

## Step 3 -- sample before you aggregate

On an unfamiliar table, prefer `SELECT ... LIMIT n` to see what the data
actually looks like before running a full `COUNT(*)` or other whole-table
aggregate -- an unbounded aggregate on a large, unfamiliar table can be
expensive and its result harder to sanity-check without having seen sample
rows first.

## Step 4 -- always bound your own result size

Always pass `max_rows` on anything that could return an unbounded result
set. Reads and writes are both allowed through `execute_sql`, and
authorization is enforced entirely by the connected role's own PostgreSQL
grants -- there's no separate statement allow-list in this tool, so use the
same judgment about destructive statements (confirm intent before any
`UPDATE`/`DELETE`/`TRUNCATE`) that you would with direct database access.

## Composing with other workflows

- Confirming table/column shape before querying overlaps with
  `postgres-schema-introspection`.
- A query that turns out to be slow, once you have real results, overlaps
  with `postgres-query-performance`.
- Whether the connected role can actually see all the rows you'd expect
  (row-level security) overlaps with `postgres-roles-permissions`.

Fetch those prompts by name for more detail rather than assuming their
content from this one.
