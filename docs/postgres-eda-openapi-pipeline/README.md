# PostgreSQL multi-version EDA to OpenAPI pipeline

This self-contained pipeline starts every maintained PostgreSQL Docker Official
Image line, introspects the live `pg_catalog` and `information_schema` catalogs,
and generates one synthetic OpenAPI 3.1 YAML file per PostgreSQL version.

The checked-in `versions.json` snapshot currently covers PostgreSQL 14, 15, 16,
17, and 18, plus the PostgreSQL 19 beta image. The version refresh tool reads
the Docker Official Image project's
[`versions.json`](https://github.com/docker-library/postgres/blob/master/versions.json)
and verifies every selected tag with `docker manifest inspect`, so a refresh
adds newly published lines and drops lines no longer maintained by that source.

All commands below assume the current directory is
`docs/postgres-eda-openapi-pipeline`.

## Prerequisites

- Docker Desktop or another running Docker daemon.
- Python 3.10 or newer.
- Enough disk space for the Alpine variants of all configured PostgreSQL images.

Create the local environment and install the generator dependencies:

```bash
cp .env.example .env
python3 -m venv .venv
.venv/bin/pip install -r tools/requirements.txt
```

The example password is only for disposable containers that do not publish a
host port. Change it if you modify the scripts to expose PostgreSQL outside the
Docker network namespace.

## Generate every version

```bash
scripts/run_all.sh
```

For each configured version, this command:

1. pulls/starts the matching `postgres:<version>-alpine` image;
2. waits for the `eda` database to accept connections;
3. extracts metadata, routines, relations, and types as JSON;
4. generates `openapi/<version>/postgres.openapi.yaml`;
5. validates the OpenAPI document when `openapi-spec-validator` is installed;
6. removes the disposable container.

Use `--stable-only` to omit prerelease images, `--keep-containers` to leave
containers running after extraction, and `--refresh` to update and verify the
official-image version manifest before running:

```bash
scripts/run_all.sh --refresh
scripts/run_all.sh --stable-only
```

## Run one version

The commands accept a manifest ID (`18`), the exact server release (`18.4`), or
the configured image (`postgres:18-alpine`):

```bash
scripts/up.sh 18
scripts/extract.sh 18
scripts/generate.sh 18
scripts/down.sh 18
```

Remove every container created by this pipeline with:

```bash
scripts/down.sh --all
```

Only containers bearing the pipeline's explicit Docker label are selected.

## Refresh Docker versions

```bash
scripts/refresh_versions.sh
```

This replaces `versions.json` with the currently maintained major lines from
the Docker Official Image source. Stable lines use floating major Alpine tags
such as `postgres:18-alpine`; prereleases use an exact tag such as
`postgres:19beta2-alpine`. The command fails without changing the manifest if
any selected image tag cannot be resolved by Docker.

Review and commit the manifest change and newly generated OpenAPI directories
after a refresh. The generated document records both the configured image and
the exact server version reported by the running container.

## Extracted data

The SQL under `sql/eda/` queries only live catalogs in the disposable database:

- `metadata.sql` records the exact server version, encoding, collation,
  selected schemas, and installed extensions.
- `routines.sql` extracts every ordinary function and procedure in
  `pg_catalog` and `information_schema`, including overload identity arguments,
  input/output modes, defaults, exact PostgreSQL types, return shape, execution
  properties, and catalog comments.
- `relations.sql` extracts readable tables, partitioned tables, views,
  materialized views, and foreign tables, including ordered column schemas.
- `types.sql` records base, composite, domain, enum, range, multirange, and
  pseudo-types for EDA and cross-version comparison.

Raw extraction files are written to `data/<version>/` and intentionally ignored
by Git. They can be regenerated from the Docker images at any time.

## OpenAPI mapping

PostgreSQL speaks its own wire protocol, so this is a synthetic API description
for schema/tool generation rather than an HTTP service exposed by PostgreSQL:

- A function/procedure overload becomes
  `POST /routines/<schema>/<name>/<signature-token>`.
- An overload's token contains a readable form of its identity arguments plus a
  deterministic hash; overloaded functions therefore never overwrite one
  another.
- Input, input/output, and variadic arguments form the JSON request body.
- Output/table arguments, composite return columns, scalar results, and set
  returns form the `200` response schema.
- A readable relation becomes `GET /relations/<schema>/<name>` with synthetic
  `limit` and `offset` query parameters and an array-of-rows response.
- Every mapped property retains its exact catalog type in `x-postgres-type` in
  addition to the closest JSON Schema type/format.
- Operation summaries and descriptions replace identifier underscores with
  spaces to produce natural-language text for semantic search; exact names stay
  unchanged in paths and PostgreSQL extensions. Operation IDs keep readable
  prefixes and a digest of the full exact identity to remain unique.
- Each operation exposes explicit `x-postgres-schema`, routine/relation kind,
  and overload identity metadata.
- `PostgresError` preserves standard diagnostic fields such as SQLSTATE,
  severity, detail, hint, source object, and constraint.

The top-level `x-postgres-authentication` extension records native password
authentication with username/password fields. It deliberately defines no
OpenAPI `securitySchemes`, because PostgreSQL connects through its own wire
protocol and negotiates the password exchange from `pg_hba.conf`.

## Layout

```text
postgres-eda-openapi-pipeline/
├── versions.json                  # maintained Docker image lines
├── scripts/                       # lifecycle, extraction, generation, validation
├── sql/eda/                       # live catalog EDA queries
├── tools/                         # version manifest and OpenAPI generators
├── tests/                         # generator/manifest unit tests
├── data/<version>/                # ignored raw JSON extraction
└── openapi/<version>/
    └── postgres.openapi.yaml      # generated OpenAPI 3.1 document
```

## Validation

```bash
python3 -m unittest discover -s tests -v
scripts/validate_all.sh
```

`validate_all.sh` also proves that every entry in `versions.json` has a
generated OpenAPI file.

## Known limitations

- Catalog rows describe built-in interfaces, but do not prove that the current
  role has permission or that every routine accepts meaningful user-supplied
  values. Internal and pseudo-types therefore remain strings with an exact
  `x-postgres-type` annotation when no faithful JSON Schema type exists.
- Default expressions are not split from `pg_proc.proargdefaults`; the pipeline
  reliably records whether an argument has a default, but not the expression.
- Relation operations describe selectable row shapes. They do not encode
  relation-specific predicates, ordering, locks, or transaction semantics.
- The generated files can be large because the pipeline intentionally preserves
  the complete callable and readable built-in catalog surface for each version.
- Docker's maintained-version list changes over time. Use
  `scripts/refresh_versions.sh` before a release when current coverage matters.
