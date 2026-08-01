# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.2.4] - 2026-07-26

### Fixed

- The disk-lock store test is now skipped on GitHub Actions' Windows runners instead of retrying longer. Three successive escalations (1s -> 10s -> 60s retry budgets, see 0.2.2 and 0.2.3) never cleared the lock on that specific CI environment, pointing at something outside the crate's own connection handling (most likely Defender or a similar background scanner holding an exclusive handle under CI load) rather than a real resource leak — the test never reproduces on any other platform or on a real Windows machine. (`28be6782`)

## [0.2.3] - 2026-07-26

### Changed

- No user-facing changes. This release closed the production test-coverage gate (76.55% -> 85.50%) — `bash scripts/coverage.sh` had been silently failing on every CI run since the gate was added, hidden behind earlier fmt/clippy failures — and widened the Windows store-lock retry window. Internal only. (`befbad32`)

## [0.2.2] - 2026-07-26

### Fixed

- Dropped the stale "18.4" PostgreSQL version from the generated-file banner/description comment (frozen at generate time before the other 5 catalog versions were added); it now reads "PostgreSQL catalog" everywhere, consistent with the crate's multi-version support. (`4a23a27b`)
- Fixed a `HOME` environment variable race in the credential-storage test suite: a test that sets the real (process-wide) `HOME` env var did so without a lock, allowing it to race an unrelated test. (`4a23a27b`)
- Widened the Windows file-lock retry budget in the store test from 1s to 10s; CI was still failing after exhausting the shorter window. (`4a23a27b`)

## [0.2.1] - 2026-07-26

### Documentation

- Corrected the README and `Cargo.toml` description, which stated "PostgreSQL 18.4" as if it were the only supported version; the crate actually embeds 6 catalogs (14, 15, 16, 17, 18, 19beta2), defaulting to 18. (`8492cd9`)

## [0.2.0] - 2026-07-26

### Added

- MCP prompts capability: a master "menu" prompt plus 9 domain-specific guided workflow prompts (schema introspection, roles & permissions, sessions & locks, replication & WAL, vacuum & maintenance, query performance, server health & config, extensions & FDW, ad hoc data profiling), so the calling LLM gets step-by-step sequencing guidance — including when to route to the `execute_sql` escape hatch — instead of re-deriving it from scratch every session. See `docs/mcp-prompts-workflow-plan.md`. (`66cb9176`)

### Fixed

- Fixed `cargo fmt --check` failures on `postgres_client.rs` and `setup_wizard.rs`. (`b9e60166`)
- Made the `test-connection` sentinel port Windows-safe: binding to `http://127.0.0.1` (port 80) collided with IIS's Default Web Site on GitHub's `windows-latest` runners, causing `test-connection` to falsely report success; now binds and drops a `TcpListener` to obtain a genuinely free port. (`b9e60166`)
- Fixed Windows handle-release timing flakiness in the store lock-release test by retrying `remove_file`. (`4dc47c5a`)

### Added

- Added a sponsor badge to the README. (`b9e60166`)

## [0.1.1] - 2026-07-26

### Added

- Initial native PostgreSQL MCP server: `search`, `get`, `call`, and `execute_sql` tools, backed by an embedded semantic catalog database and PostgreSQL's native wire protocol (rather than HTTP) for live calls. Includes setup wizard, credential storage (OS keychain with encrypted-file fallback), structured logging, OpenTelemetry tracing, HTTP/stdio transports, health checks, and Docker packaging. (`d7713628`)
- Added the PostgreSQL multi-version EDA-to-OpenAPI pipeline (`docs/postgres-eda-openapi-pipeline`): introspects live `pg_catalog`/`information_schema` catalogs across every maintained PostgreSQL Docker Official Image line (14 through the 19 beta) and generates one synthetic OpenAPI 3.1 document per version, used as the generation source for the embedded catalogs above. (`442fbb16`)
- Initial project scaffold (license, `.gitignore`, README stub). (`8b7fd159`)

### Fixed

- Catalog operation schemas stored in `mcp_store.db` now fully inline every `$ref` instead of leaving unresolved local `#/$defs/...` references. Added a `POSTGRES_MCP_READ_ONLY` safeguard (default `true`) for `execute_sql`: when enabled, the session is placed into PostgreSQL's own `default_transaction_read_only` mode before any statement runs, so the database itself rejects writes — independent of the connecting role's grants, and covering writes smuggled inside a data-modifying CTE under an outer `SELECT`. (`376a003c`)
