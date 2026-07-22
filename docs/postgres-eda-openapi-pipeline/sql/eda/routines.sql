WITH catalog_routines AS (
    SELECT
        p.oid,
        n.nspname AS schema_name,
        p.proname AS routine_name,
        CASE p.prokind WHEN 'p' THEN 'procedure' ELSE 'function' END AS routine_kind,
        pg_get_function_identity_arguments(p.oid) AS identity_arguments,
        pg_get_function_result(p.oid) AS result_type,
        p.proretset AS returns_set,
        p.provolatile::text AS volatility,
        p.proparallel::text AS parallel_safety,
        p.prosecdef AS security_definer,
        p.proleakproof AS leakproof,
        p.proisstrict AS strict,
        p.pronargs,
        p.pronargdefaults,
        p.proargtypes,
        p.proallargtypes,
        p.proargmodes,
        p.proargnames,
        p.prorettype,
        obj_description(p.oid, 'pg_proc') AS description
    FROM pg_proc AS p
    JOIN pg_namespace AS n ON n.oid = p.pronamespace
    WHERE n.nspname IN ('pg_catalog', 'information_schema')
      AND p.prokind IN ('f', 'p')
),
expanded AS (
    SELECT
        routine.*,
        argument.ordinal,
        coalesce(
            nullif(routine.proargnames[argument.ordinal], ''),
            'arg' || argument.ordinal::text
        ) AS argument_name,
        coalesce(routine.proargmodes[argument.ordinal], 'i')::text AS argument_mode,
        format_type(argument.type_oid, NULL) AS argument_type,
        count(*) FILTER (
            WHERE coalesce(routine.proargmodes[argument.ordinal], 'i') IN ('i', 'b', 'v')
        ) OVER (PARTITION BY routine.oid ORDER BY argument.ordinal) AS input_position
    FROM catalog_routines AS routine
    LEFT JOIN LATERAL unnest(
        coalesce(routine.proallargtypes, routine.proargtypes::oid[])
    ) WITH ORDINALITY AS argument(type_oid, ordinal) ON true
),
assembled AS (
    SELECT
        routine.oid,
        routine.schema_name,
        routine.routine_name,
        routine.routine_kind,
        routine.identity_arguments,
        routine.result_type,
        routine.returns_set,
        routine.volatility,
        routine.parallel_safety,
        routine.security_definer,
        routine.leakproof,
        routine.strict,
        routine.description,
        coalesce((
            SELECT jsonb_agg(
                jsonb_build_object(
                    'name', argument.argument_name,
                    'mode', CASE argument.argument_mode
                        WHEN 'i' THEN 'in'
                        WHEN 'o' THEN 'out'
                        WHEN 'b' THEN 'inout'
                        WHEN 'v' THEN 'variadic'
                        WHEN 't' THEN 'table'
                    END,
                    'data_type', argument.argument_type,
                    'ordinal', argument.ordinal,
                    'has_default', argument.argument_mode IN ('i', 'b', 'v')
                        AND argument.input_position > argument.pronargs - argument.pronargdefaults
                ) ORDER BY argument.ordinal
            )
            FROM expanded AS argument
            WHERE argument.oid = routine.oid
              AND argument.ordinal IS NOT NULL
        ), '[]'::jsonb) AS arguments,
        coalesce((
            SELECT jsonb_agg(
                jsonb_build_object(
                    'name', attribute.attname,
                    'data_type', format_type(attribute.atttypid, attribute.atttypmod),
                    'nullable', NOT attribute.attnotnull,
                    'ordinal', attribute.attnum
                ) ORDER BY attribute.attnum
            )
            FROM pg_type AS result_type
            JOIN pg_attribute AS attribute ON attribute.attrelid = result_type.typrelid
            WHERE result_type.oid = routine.prorettype
              AND attribute.attnum > 0
              AND NOT attribute.attisdropped
        ), '[]'::jsonb) AS return_columns
    FROM catalog_routines AS routine
)
SELECT coalesce(
    jsonb_agg(
        to_jsonb(assembled) - 'oid'
        ORDER BY schema_name, routine_name, identity_arguments
    ),
    '[]'::jsonb
)::text
FROM assembled;
