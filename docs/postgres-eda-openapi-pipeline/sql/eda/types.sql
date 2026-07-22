WITH catalog_types AS (
    SELECT
        namespace.nspname AS schema_name,
        type.typname AS type_name,
        CASE type.typtype
            WHEN 'b' THEN 'base'
            WHEN 'c' THEN 'composite'
            WHEN 'd' THEN 'domain'
            WHEN 'e' THEN 'enum'
            WHEN 'm' THEN 'multirange'
            WHEN 'p' THEN 'pseudo'
            WHEN 'r' THEN 'range'
        END AS type_kind,
        format_type(type.oid, NULL) AS formatted_type,
        format_type(type.typbasetype, type.typtypmod) AS base_type,
        type.typnotnull AS not_null,
        obj_description(type.oid, 'pg_type') AS description,
        coalesce((
            SELECT jsonb_agg(enum.enumlabel ORDER BY enum.enumsortorder)
            FROM pg_enum AS enum
            WHERE enum.enumtypid = type.oid
        ), '[]'::jsonb) AS enum_values
    FROM pg_type AS type
    JOIN pg_namespace AS namespace ON namespace.oid = type.typnamespace
    WHERE namespace.nspname IN ('pg_catalog', 'information_schema')
)
SELECT coalesce(
    jsonb_agg(catalog_types ORDER BY schema_name, type_name),
    '[]'::jsonb
)::text
FROM catalog_types;
