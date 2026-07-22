WITH catalog_relations AS (
    SELECT
        class.oid,
        namespace.nspname AS schema_name,
        class.relname AS relation_name,
        CASE class.relkind
            WHEN 'r' THEN 'table'
            WHEN 'p' THEN 'partitioned table'
            WHEN 'v' THEN 'view'
            WHEN 'm' THEN 'materialized view'
            WHEN 'f' THEN 'foreign table'
        END AS relation_kind,
        obj_description(class.oid, 'pg_class') AS description
    FROM pg_class AS class
    JOIN pg_namespace AS namespace ON namespace.oid = class.relnamespace
    WHERE namespace.nspname IN ('pg_catalog', 'information_schema')
      AND class.relkind IN ('r', 'p', 'v', 'm', 'f')
),
assembled AS (
    SELECT
        relation.schema_name,
        relation.relation_name,
        relation.relation_kind,
        relation.description,
        coalesce(jsonb_agg(
            jsonb_build_object(
                'name', attribute.attname,
                'data_type', format_type(attribute.atttypid, attribute.atttypmod),
                'nullable', NOT attribute.attnotnull,
                'ordinal', attribute.attnum,
                'description', col_description(relation.oid, attribute.attnum)
            ) ORDER BY attribute.attnum
        ) FILTER (WHERE attribute.attnum IS NOT NULL), '[]'::jsonb) AS columns
    FROM catalog_relations AS relation
    LEFT JOIN pg_attribute AS attribute
      ON attribute.attrelid = relation.oid
     AND attribute.attnum > 0
     AND NOT attribute.attisdropped
    GROUP BY relation.oid, relation.schema_name, relation.relation_name,
             relation.relation_kind, relation.description
)
SELECT coalesce(
    jsonb_agg(assembled ORDER BY schema_name, relation_name),
    '[]'::jsonb
)::text
FROM assembled;
