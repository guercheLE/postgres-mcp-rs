SELECT jsonb_build_object(
    'server_version', current_setting('server_version'),
    'server_version_num', current_setting('server_version_num')::integer,
    'database', current_database(),
    'server_encoding', current_setting('server_encoding'),
    'lc_collate', (
        SELECT datcollate FROM pg_database WHERE datname = current_database()
    ),
    'schemas', (
        SELECT coalesce(jsonb_agg(nspname ORDER BY nspname), '[]'::jsonb)
        FROM pg_namespace
        WHERE nspname IN ('pg_catalog', 'information_schema')
    ),
    'extensions', (
        SELECT coalesce(
            jsonb_agg(
                jsonb_build_object('name', extname, 'version', extversion)
                ORDER BY extname
            ),
            '[]'::jsonb
        )
        FROM pg_extension
    )
)::text;
