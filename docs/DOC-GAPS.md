# Documentation gaps

Retrospective mined from every commit in this repo's git history (17 commits,
6 releases, `v0.1.1` through `v0.2.4`, all dated 2026-07-21/22/26), looking
for `fix:` commits that imply a design/doc gap and `feat:` commits that
retrofitted something a planning doc should have anticipated.

**Scope note:** this repo does not have a full PRD/architecture doc set. Its
only planning-adjacent docs are `docs/SCHEMA_VERSIONS.md` (catalog-version
reference table), `docs/mcp-prompts-workflow-plan.md` (a retrofit plan
written after the fact, see below), and `docs/postgres-eda-openapi-pipeline/README.md`
(the OpenAPI-generation pipeline's own README). The README states the
project's scope directly. That narrower baseline changes *which* docs each
gap below is compared against — it does not change how thoroughly the git
log itself was mined; every one of the 17 commits was reviewed.

Repo is young (6 releases over essentially one week of activity), so the
gap count below is modest. That's reported honestly rather than padded.

**Sourcing note for `(shared mcpify-template gap)` entries:** this repo is
generated output from `mcpify` (source at
`/Users/lucianoguerche/Documents/GitHub/mcpify`), whose own planning docs
live at `mcpify/docs/`: `product-brief.md`, `prd.md`, `architecture.md`, and
`v1-implementation-plan.md` through `v10-implementation-plan.md`. For every
entry below tagged `(shared mcpify-template gap)`, the "Doc gap" section
cites the specific mcpify doc (and, where findable, the section/line) that
should have specified the missing behavior — not just the absence of
coverage in this repo's own local docs. mcpify's main session independently
ran the identical retrospective methodology directly against mcpify's own
git history, producing `mcpify/docs/DOC-GAPS.md` and `mcpify/CHANGELOG.md`.
Where a shared-gap entry below matches a root cause already catalogued
there, it says so explicitly and points at the matching entry, rather than
re-deriving a differently-worded version of the same finding. Where a
shared-gap entry below has **no** match in mcpify's own `docs/DOC-GAPS.md`,
that's flagged as a genuinely new finding — one this repo's git history
surfaced that mcpify's own retrospective (grounded in mcpify's git history)
did not, and that may be worth a fresh entry there. Entries tagged
`(repo-specific)` are compared only against this repo's own local docs, as
before — they're genuinely about PostgreSQL-specific domain behavior, not
generator gaps.

## Lessons for future docs

1. **Give every write-capable "escape hatch" tool an explicit safety-default
   decision in the first planning doc, not a follow-up fix.** `execute_sql`
   shipped read/write-by-default in the very first feature commit
   (`d7713628`), and only got a documented, opt-out `read_only` safeguard
   four days later after presumably being flagged as risky (`376a003c`). This
   is genuinely repo-specific — mcpify's own `docs/prd.md`/`architecture.md`
   define only `search`/`get`/`call` as the universal tool surface and never
   mention a raw-execution escape hatch at all, so there was no generator-level
   doc to fall down on here. Any future MCP server (or any tool, generated or
   hand-built) that exposes raw query/command execution should have its
   default safety posture — read-only unless explicitly opted into writes —
   decided and written down *before* v1 ships, not retrofitted.

2. **When a doc asserts an invariant ("schemas are fully self-contained"),
   verify the generator actually satisfies it before the doc ships, and wire
   an automated check for it immediately.** This repo's `docs/SCHEMA_VERSIONS.md`
   claimed full `$ref` self-containment from its very first commit, but the
   generator output didn't yet satisfy that claim until a later fix
   (`376a003c`). Root-caused one level up: mcpify's own `docs/prd.md` REQ-2.4.1
   and `docs/architecture.md` §2 ("Data Layer") call `mcp_store.db`
   "self-contained" without ever specifying what that should mean at scale —
   whole-library vs. reachable-only schemas, `$ref` inlined vs. localized into
   `$defs`. mcpify's own `docs/DOC-GAPS.md` already catalogues this exact gap
   (see the cross-referenced entry below); the lesson generalizes: docs that
   describe a generated artifact's guarantees should ship together with the
   test that proves the guarantee, not before it, and the guarantee's *precise
   shape* (not just "self-contained") needs to be spelled out once, centrally.

3. **Document that any version-numbered generated file/string is either
   re-derived on every `add-version` call, or written in a version-neutral
   way from the start — and say which, explicitly.** This repo hit staleness
   twice, in two different files, across two separate releases: README/Cargo.toml
   still saying "PostgreSQL 18.4" as the only version (`8492cd9`, v0.2.1),
   then the generated-file banner comment sourced from `.mcpify/versions.json`'s
   `display_name` making the same stale claim (`4a23a27`, v0.2.2). This is
   *not* an omission in mcpify's docs — `docs/architecture.md` (line 240) and
   `docs/v8-implementation-plan.md` (line 15) both explicitly, deliberately
   decide that `display_name`/`project_name`/`language` are "written once by
   `generate` and never re-derived from a later spec... so `add-version` never
   churns unrelated file headers." That's a reasonable trade-off on its own
   terms, but neither doc ever flags its consequence: any *other* generated
   text a template author writes that happens to reference the version number
   (a banner comment, README prose, a package description) will go stale the
   moment `add-version` is used, unless that text is deliberately phrased
   version-neutrally up front. The fix isn't "stop freezing `display_name`"
   (that would reintroduce the churn the freeze was designed to avoid) — it's
   "document the freeze's blast radius so every template author writing
   version-referencing prose knows to route around it, not just the `.mcpify/versions.json`
   consumers who already know about the freeze." Neither mcpify's own
   `docs/DOC-GAPS.md` nor its architecture doc currently says this — see the
   two entries below flagged as new findings.

4. **Document platform-specific file-locking behavior and a retry/backoff
   policy up front for any embedded on-disk store.** This repo's Windows CI
   file-lock flakiness for `store.rs`'s test was fixed four separate times
   across four releases, each escalating a retry budget (1s → 10s → 60s)
   before finally giving up and skipping the test on GitHub Actions' Windows
   runners entirely (`4dc47c5`, `4a23a27`, `befbad3`, `28be6782`) — and this
   is a verbatim repeat of mcpify's *own* nine-fix saga for the identical
   problem in its own generator/CI (`mcpify/docs/DOC-GAPS.md`,
   "Windows-specific file-locking/timing semantics unaddressed for the
   embedded store," v0.10.5–v0.11.11): mcpify's `docs/architecture.md` §2
   ("Data Layer") describes `mcp_store.db` extraction with zero mention of
   concurrency or platform-specific file-handle semantics, and this repo
   inherited that same blind spot in its own hand-copied test. A doc that
   states up front "Windows CI runners intermittently hold file handles
   longer than expected (Defender-class scanners are the leading suspect);
   retry budgets should be generous from day one, and platform-specific
   skips are an acceptable and expected outcome, not a workaround to avoid"
   — written once, centrally, in mcpify's architecture doc — would have saved
   both this repo's four-fix cycle *and* mcpify's own nine-fix cycle.

5. **Document test-isolation rules for anything that mutates real
   process-wide state.** `credential_storage.rs`'s test suite mutated the
   real `$HOME` environment variable with no lock, racing any other test
   thread reading it (`4a23a27`) — the identical bug mcpify fixed in its own
   codebase the same day (v0.11.9, "Fixed a test race caused by multiple
   tests locking on `$HOME` concurrently"). No testing-conventions doc exists
   in either repo to say "tests touching global process state need an
   explicit lock or must not run in parallel with anything reading that same
   state" — that belongs in a generator-level testing-conventions doc (there
   isn't one in mcpify's `docs/` today) so every downstream repo doesn't
   rediscover it independently through a flaky CI run.

6. **A CI gate that never actually ran is worse than no gate — verify gates
   execute, not just that they exist.** This repo's production coverage gate
   had been silently failing on every CI run since it was added, masked
   behind earlier fmt/clippy failures, until `befbad3` (v0.2.3) both fixed
   the masking issues and closed the real gap (76.55% → 85.50%). This one
   does *not* have a matching entry in mcpify's own `docs/DOC-GAPS.md` —
   flagged below as a new finding, generalized beyond this one repo since
   every mcpify-generated project's CI runs the same fmt → clippy → test →
   coverage step order. A CI/release doc should state the expected baseline
   for each gate and flag a gate that hasn't moved or hasn't reported in N
   runs, not just add the gate and assume it's working.

7. **Decide, in the very first planning doc, whether a flat tool surface
   needs guided workflow prompts — and if the pattern is proven, put it in
   the generator, not in each downstream repo by hand.** This repo's flat
   `search`/`get`/`call`/`execute_sql` surface left all step-by-step
   sequencing knowledge to the calling LLM until `docs/mcp-prompts-workflow-plan.md`
   was written and `66cb9176` retrofitted 10 MCP prompts after initial
   release. That plan explicitly says the sibling `jira-dc-mcp-rs` project
   (same generator, same architecture) had already solved the identical
   problem the same way — and a search of mcpify's entire `docs/` tree (PRD,
   architecture, product brief, all ten `vN-implementation-plan.md` files)
   turns up *zero* mentions of an MCP "prompts" capability. This means at
   least two downstream repos have now hand-rolled the identical feature
   entirely outside the generator, with no record of it in mcpify's own
   planning docs or gap retrospective — see the new-finding entry below. When
   the same non-trivial feature gets copy-pasted "verbatim where domain-agnostic"
   across multiple generated repos, that's the signal to promote it into the
   generator's own templates instead of leaving every future repo to
   rediscover the need and hand-author it again.

## Doc gap entries

### [0.2.4] 2026-07-26 — Windows disk-lock test skipped instead of retried further (shared mcpify-template gap)

#### Doc gap
mcpify's `docs/architecture.md` §2 ("Data Layer") documents the single-file
`mcp_store.db` design decision but never mentions concurrency or
platform-specific file-handle semantics; neither it nor `docs/prd.md`
REQ-2.4.1 states that store extraction/rename must be safe under concurrent
calls, or that Windows releases file handles on a different timeline than
POSIX. No doc anywhere (this repo's or mcpify's) specifies a decision policy
for "how long should Windows-CI-only file-lock flakiness be retried before
concluding retrying won't help and the test should be platform-skipped
instead." **This matches mcpify's own `docs/DOC-GAPS.md` entry "2026-07-19 to
2026-07-26 - Windows-specific file-locking/timing semantics unaddressed for
the embedded store (v0.10.5 - v0.11.11)"** — and this repo's fix is
essentially the same fix mcpify shipped to its own generator/CI the same day,
v0.11.11 ("fix: skip the disk-lock test on GitHub Actions' Windows runners,
not just retry longer"), applied by hand to this already-generated repo's
own copy of the test rather than inherited from a regenerated template. This
was the fourth and final escalation of the same underlying gap in this repo
(see the v0.2.0 and v0.2.2 entries below).

#### Resulting work
Added a `GITHUB_ACTIONS`+`windows` runtime skip guard around the disk-lock
test in `src/data/store.rs`, matching the skip pattern `tests/cli_smoke.rs`
already used for its own runner-specific case; kept a short local retry for
genuine transient contention on platforms where it actually clears.

---

### [0.2.3] 2026-07-26 — Windows store-lock retry widened again, 10s → 60s (shared mcpify-template gap)

#### Doc gap
Same root cause as the v0.2.4 entry above (mcpify's `docs/architecture.md`
§2 never addresses store-file concurrency/platform semantics) — third
escalation of the same issue in this repo, still not enough. **Matches
mcpify's own `docs/DOC-GAPS.md` "Windows-specific file-locking/timing
semantics" entry**, whose resulting-work list includes the matching upstream
fix shipped the same day: v0.11.10 (2026-07-26), "Widened the Windows
store-lock retry window to 60 seconds."

#### Resulting work
Widened the `store.rs` lock-release test's retry budget from 100×100ms
(10s) to 600×100ms (60s), bundled into the same commit as the coverage-gate
fix below.

---

### [0.2.3] 2026-07-26 — Production coverage gate had been silently broken since it was added (shared mcpify-template gap)

#### Doc gap
mcpify's `docs/v6-implementation-plan.md` (GEN1, "Coverage via
`cargo-llvm-cov`") specifies how mcpify's own coverage tooling
(`scripts/coverage.sh`) should be built, and the 85%-production-coverage
requirement for generated Rust projects traces to mcpify's own v0.10.0
release — but no doc anywhere in mcpify's `docs/` (`prd.md`,
`architecture.md`, or any `vN-implementation-plan.md`) ever states that a
CI coverage-gate step must be independently verified to actually execute
and report, separate from earlier steps in the same pipeline. `bash
scripts/coverage.sh` had been failing on every CI run in this repo since the
gate was introduced, hidden behind earlier `fmt`/`clippy` failures that
masked it — `src/cli/execute_sql.rs`, `src/tools/call_tool.rs`, and
`src/tools/execute_sql_tool.rs` sat at 0% coverage, and
`services/postgres_client.rs` (391 lines) at 47%, all undetected. **This is
a genuinely new finding: mcpify's own `docs/DOC-GAPS.md` has no entry for
"a CI gate can silently no-op behind an earlier failing step."** Since every
mcpify-generated project's CI runs the same fmt → clippy → test → coverage
step order, this masking risk is structural to the generator's CI template,
not PostgreSQL-specific — worth a fresh entry in mcpify's own
`docs/DOC-GAPS.md`.

#### Resulting work
Added a large batch of unit/integration tests (pure-helper unit tests for
`postgres_client.rs`, connection-refused async tests exercising the
network-attempt boundary, `call_tool.rs`/`mcp_server.rs`/CLI validation
coverage, credential round-trip and `from_hex` odd-length tests) to close
coverage from 76.55% to 85.50%.

---

### [0.2.2] 2026-07-26 — Stale "18.4" version banner in generated-file comments (shared mcpify-template gap)

#### Doc gap
mcpify's `docs/architecture.md` (line 240) and `docs/v8-implementation-plan.md`
(line 15) both **explicitly and deliberately** document that
`.mcpify/versions.json`'s `display_name`/`project_name`/`language` fields
"are written once by `generate` and never re-derived from a later spec, so
`add-version` never churns unrelated file headers." So this isn't an
omission in mcpify's docs — it's a documented, intentional trade-off (avoid
regeneration churn). What neither doc ever addresses is the *consequence*:
any other generated text that happens to reference the version number (this
repo's per-file banner comment, sourced from that same frozen `display_name`)
will silently go stale the moment `add-version` adds more versions, unless
that text was written in a version-neutral way from the very first
`generate` call. This is the same root cause as the v0.2.1 README/Cargo.toml
entry below, surfacing a second time in a different generated location.
**This precise consequence — not the freeze itself — has no entry in
mcpify's own `docs/DOC-GAPS.md`; it's a genuinely new finding**, worth
folding into mcpify's architecture doc as explicit guidance for template
authors: any generated prose that could outlive a single spec version must
either be phrased version-neutrally or be added to the marker-delimited
"version-aware regions" `add-version` already knows how to re-render.

#### Resulting work
Reworded the generated-file banner/description comment (present at the top
of every generated `.rs` file) from the stale "PostgreSQL 18.4" to the
version-neutral "PostgreSQL catalog," matching the crate's actual
multi-version support.

---

### [0.2.2] 2026-07-26 — `$HOME` env var race in credential-storage tests (shared mcpify-template gap)

#### Doc gap
No documented test-isolation convention exists in mcpify's `docs/` (there is
no testing-conventions doc there at all, and none in this repo either) for
tests that mutate real process-wide environment state. `credential_storage.rs`'s
`file_fallback_round_trips_a_credential` test set the real `$HOME`
environment variable — process-wide state every test thread shares —
without any lock, so it could race any other test reading `$HOME`
concurrently. **Matches mcpify's own `docs/DOC-GAPS.md` "Windows-specific
file-locking/timing semantics" entry**, whose resulting-work list includes
the identical fix landing in mcpify's own codebase the same day: v0.11.9
(2026-07-26), "Fixed a test race caused by multiple tests locking on `$HOME`
concurrently." Worth noting the topical mismatch even in mcpify's own
retrospective: this specific race is a general test-isolation gap, not a
Windows-file-locking one — it's filed there only because it shipped bundled
in the same release as the Windows store-lock retry widen below.

#### Resulting work
Added a dedicated `HOME_ENV_TEST_LOCK` mutex around the test.

---

### [0.2.2] 2026-07-26 — Windows store-lock retry widened, 1s → 10s (shared mcpify-template gap)

#### Doc gap
Same root cause as the v0.2.4 and v0.2.3 entries above (mcpify's
`docs/architecture.md` §2 never addresses store-file concurrency/platform
semantics) — second escalation of the same issue in this repo. **Matches
mcpify's own `docs/DOC-GAPS.md` "Windows-specific file-locking/timing
semantics" entry**, bundled into the same upstream release as the `$HOME`
race fix above: v0.11.9 (2026-07-26), "Widened Windows retry windows
further for store-lock contention."

#### Resulting work
Widened the `store.rs` lock-release test's retry budget from 50×20ms (1s)
to 100×100ms (10s); still not enough (see v0.2.3 above).

---

### [0.2.1] 2026-07-26 — README/Cargo.toml claimed only PostgreSQL 18.4 is supported (shared mcpify-template gap)

#### Doc gap
`docs/SCHEMA_VERSIONS.md` correctly listed all 6 embedded catalog versions
from its very first commit, but the top-level README and the `Cargo.toml`
package description — the first things a user or crates.io visitor
actually reads — still said "PostgreSQL 18.4" as if it were the sole
supported version. Same root cause as the v0.2.2 `display_name` entry above:
mcpify's `docs/architecture.md` §5 ("Multi-Version Spec Support") and
`docs/v8-implementation-plan.md` fully document the mechanics of
`add-version` (the ledger, the marker-delimited "version-aware regions" it
re-renders) but never state that hand-written, non-marker-region prose
describing "supported version(s)" — like this repo's top-level README intro
and `Cargo.toml` `description` field, neither of which is one of the
version-aware regions `add-version` knows to re-render — needs to either be
added to that re-rendered set or written version-neutrally from the start.
**New finding, not yet in mcpify's own `docs/DOC-GAPS.md`**: the two
entries here (this one and the `display_name` banner above) are really one
underlying gap that surfaced independently in two different generated files
across two different releases of this repo.

#### Resulting work
Corrected the README and `Cargo.toml` wording to list all 6 supported
versions (14 through 19beta2, defaulting to 18).

---

### [0.2.0] 2026-07-26 — MCP prompts capability retrofitted after initial release (shared mcpify-template gap)

#### Doc gap
No planning doc for this repo (there is no PRD/architecture doc set at all)
decided, before v0.1.1 shipped, whether a flat 4-tool (`search`/`get`/
`call`/`execute_sql`) surface should also ship guided "prompts" workflows in
v1. Root-caused one level up: a full-text search of every mcpify planning
doc — `docs/prd.md`, `docs/architecture.md`, `docs/product-brief.md`, and
all ten `docs/vN-implementation-plan.md` files — for "prompts",
"prompt_router", and "guided workflow" turns up **zero** hits describing an
MCP prompts capability. mcpify's core tool-surface requirement defines only
`search`/`get`/`call` (plus per-target extras like this repo's own
`execute_sql`); nothing anywhere in mcpify's planning docs ever asks
whether a flat, sequencing-agnostic tool surface needs guided prompts to be
usable independently by an LLM client. The gap was significant enough in
this repo that closing it required writing an entirely new 245-line
planning document (`docs/mcp-prompts-workflow-plan.md`) after the fact. That
plan explicitly states the sibling `jira-dc-mcp-rs` project — same mcpify
generator, same flat-tools-over-an-embedded-catalog architecture — had
already solved the identical problem the same way, and that this plan
replicates its pattern "verbatim where it's domain-agnostic." **This is a
genuinely new finding, not covered anywhere in mcpify's own
`docs/DOC-GAPS.md`**: at least two downstream repos have now hand-rolled the
identical, non-trivial feature entirely outside the generator, with no
record of the need in mcpify's own planning docs or gap retrospective —
strong signal the pattern belongs in mcpify's generator/templates rather
than being copy-pasted repo-by-repo, and worth a fresh mcpify
`docs/DOC-GAPS.md` entry.

#### Resulting work
Added a new `src/prompts/` module: 1 master menu prompt + 9 domain-specific
guided workflow prompts, wired into `McpifyServer` via a second
`#[prompt_router]`-decorated `impl` block and a new `.enable_prompts()`
capability flag, plus `tests/prompts_workflow.rs` for protocol-level
coverage.

---

### [0.2.0] 2026-07-26 — Windows-unsafe test-connection sentinel port (shared mcpify-template gap)

#### Doc gap
mcpify's `docs/prd.md` REQ-2.3.4 requires a `test-connection` CLI command
(and `docs/architecture.md` lists it in the CLI's subcommand set) but
neither doc ever specifies how its connectivity-check sentinel should be
chosen — no cross-platform port-selection convention is documented anywhere
for this class of generated test helper. The generated `test-connection`
test helper (`generated_server()`) bound to `http://127.0.0.1` (implicit
port 80) as a connectivity sentinel; nothing documented that port 80
collides with IIS's Default Web Site on GitHub's `windows-latest` runners,
which made the test wrongly report success there. The underlying fix
already shipped in mcpify itself the same day, per mcpify's own
`CHANGELOG.md` v0.11.8: "Switched the `cli_smoke` test's sentinel URL to an
ephemeral port to avoid port collisions in CI" — so this is not an
open/unfixed gap in mcpify's generator. **However, mcpify's own
`docs/DOC-GAPS.md` doesn't name this issue as its own topic**: it's bundled
anonymously inside the "Windows-specific file-locking/timing semantics"
entry's resulting-work list (v0.11.8), alongside the topically distinct
`remove_file` retry fix from the same release. Port-selection-for-sentinels
is a different bug class from file-lock timing (an availability collision
with a well-known service, not a handle-release race), and no doc — this
repo's, this repo's local docs, or mcpify's retrospective — names it
separately. Partial new finding: worth a one-line split in mcpify's existing
entry, or its own short entry, distinguishing the two root causes.

#### Resulting work
Changed the sentinel to bind and drop a `TcpListener` to obtain a genuinely
free ephemeral port instead of a hardcoded one; bundled with an unrelated
`cargo fmt --check` fix and a README sponsor-badge addition in the same
commit.

---

### [0.2.0] 2026-07-26 — Windows handle-release timing flakiness in store lock-release test (shared mcpify-template gap)

#### Doc gap
First occurrence, in this repo, of the Windows file-lock timing gap
described in the v0.2.2/v0.2.3/v0.2.4 entries above. Root cause: mcpify's
`docs/architecture.md` §2 ("Data Layer") never documents an expectation that
Windows CI runners can hold a file handle open longer than the moment a
Rust-level drop/close call returns. **Matches mcpify's own `docs/DOC-GAPS.md`
"Windows-specific file-locking/timing semantics" entry** directly — its
resulting-work list's v0.11.8 (2026-07-26) line reads verbatim the same as
this repo's fix: "Added a retry around `remove_file` in the store lock-release
test to tolerate Windows' slower file-handle release timing." This repo was
generated 2026-07-22, after mcpify's first four fix iterations for this bug
class (v0.8.2 on 07-16, v0.10.5/v0.11.1 on 07-19, v0.11.2 on 07-21) were
already folded into the template, but mcpify then needed *four more*
iterations (v0.11.8–v0.11.11) starting 07-26 — all after this repo already
existed. Because this repo hand-adapts the generated dispatcher for
PostgreSQL's native protocol (its own README warns "re-running the generic
HTTP-oriented generator will overwrite the native dispatcher"), it can't
just take a regenerated diff to pick up template fixes — this repo's own
commits had to independently re-discover and re-apply the same four-stage
fix sequence in its own copy of the test, landing on the same calendar days
as mcpify's own fixes.

#### Resulting work
Added a retry loop around `remove_file` in the store lock-release test.

---

### [0.1.1] 2026-07-26 — Catalog schemas left unresolved `$ref`s despite the docs already claiming otherwise (shared mcpify-template gap)

#### Doc gap
`docs/SCHEMA_VERSIONS.md` was written at initial generation time already
asserting "every literal input/output schema stored in SQLite is
self-contained... rebased to local `#/$defs/...` references with their
transitive definitions embedded." The actual generator output didn't yet
satisfy that claim: schemas still referenced unresolved `#/$defs/...`
fragments (e.g. `#/$defs/PostgresError`). Root-caused one level up: mcpify's
own `docs/prd.md` REQ-2.4.1 and `docs/architecture.md` §2 ("Data Layer")
both call `mcp_store.db` "self-contained" without ever specifying what that
guarantee should mean at scale — whole-library vs. reachable-only schemas,
`$ref` fully inlined vs. localized into `$defs`. **This matches mcpify's own
`docs/DOC-GAPS.md` entry "2026-07-21 to 2026-07-26 - self-contained
schema/`$ref` handling took three iterations (v0.5.12, v0.11.5, v0.11.7)"**
precisely — the fix commit here ties its root cause directly to "mcpify's
earlier $ref-to-$defs-localization fix" and states it "matches a companion
fix in the mcpify generator itself." This repo's fix (376a003c) is the same
change as mcpify's own v0.11.7, "fix(openapi): fully inline $ref instead of
localizing into $defs — the third and final correction," landed the same
day (2026-07-26) — the shared generator's own `$ref`-resolution logic was
incomplete through two prior iterations (v0.5.12, v0.11.5), and every repo
generated before the third and final generator-level fix inherited output
that didn't match what its own doc already (prematurely) promised.

#### Resulting work
Recursively inlined every `$ref` in the embedded catalog schemas (verified
no genuine reference cycles exist in this catalog); extended
`tests/schema_references.rs` to exhaustively check the invariant for every
version going forward.

---

### [0.1.1] 2026-07-26 — `execute_sql` shipped without an opt-in safety default (repo-specific)

#### Doc gap
No doc — not the README, not any planning doc, since none existed at
initial release — specified that a raw parameterized-SQL escape-hatch tool
needed a safe-by-default read-only posture before shipping. The initial
`feat: add native PostgreSQL MCP server` commit shipped `execute_sql` as
unrestricted read/write from day one, documented only as relying on "the
connected role's grants" as the safety boundary. Nothing flagged that a
data-modifying CTE nested under an outer `SELECT` (e.g. `WITH t AS
(DELETE ... RETURNING *) SELECT * FROM t`) can smuggle a write past
app-level SQL-text sniffing, or that adopters might expect a safe-by-default
mode to exist. This is PostgreSQL-specific (the `default_transaction_read_only`
session mechanism used to enforce it is a native PostgreSQL feature, not a
generic mcpify pattern), so it's a genuine gap in this repo's own docs
rather than a shared template issue.

#### Resulting work
Added `POSTGRES_MCP_READ_ONLY` (default `true`): when enabled, every
`execute_sql` connection is placed into PostgreSQL's own
`default_transaction_read_only` session mode before any statement runs, so
the database itself rejects writes independent of the connecting role's
actual grants. `call` operations were unaffected — they already always ran
inside their own forced read-only transaction.
