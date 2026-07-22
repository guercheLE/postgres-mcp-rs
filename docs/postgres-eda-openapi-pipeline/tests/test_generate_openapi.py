import importlib.util
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parent.parent
SPEC = importlib.util.spec_from_file_location(
    "generate_openapi", ROOT / "tools" / "generate_openapi.py"
)
generate_openapi = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(generate_openapi)


class GenerateOpenApiTests(unittest.TestCase):
    def test_postgres_type_mapping_preserves_exact_type(self):
        self.assertEqual(
            {"type": "array", "items": {"type": "integer", "format": "int32",
             "x-postgres-type": "integer"},
             "x-postgres-type": "integer[]"},
            generate_openapi.postgres_type_to_schema("integer[]"),
        )
        self.assertEqual(
            {"x-postgres-type": "jsonb"},
            generate_openapi.postgres_type_to_schema("jsonb"),
        )
        self.assertEqual(
            "integer",
            generate_openapi.argument_schema(
                {"mode": "variadic", "data_type": "integer[]"}
            )["items"]["x-postgres-type"],
        )
        self.assertEqual(
            "arg2", generate_openapi.argument_name({"name": "", "ordinal": 2})
        )
        self.assertEqual(
            {"type": "null", "x-postgres-type": "void"},
            generate_openapi.allow_null(
                {"type": "null", "x-postgres-type": "void"}
            ),
        )
        nullable_row = generate_openapi.object_schema(
            [{"name": "value", "data_type": "text", "nullable": True, "ordinal": 1}]
        )
        self.assertEqual(["string", "null"], nullable_row["properties"]["value"]["type"])
        self.assertEqual(["value"], nullable_row["required"])

    def test_signature_token_is_deterministic_and_overload_safe(self):
        integer = generate_openapi.signature_token("integer")
        text = generate_openapi.signature_token("text")

        self.assertEqual(integer, generate_openapi.signature_token("integer"))
        self.assertNotEqual(integer, text)
        self.assertRegex(integer, r"^[a-z0-9_]+_[0-9a-f]{8}$")

        first = generate_openapi.unique_operation_id(
            "call", "pg_catalog", "a_very_long_routine_name" * 4, "integer"
        )
        second = generate_openapi.unique_operation_id(
            "call", "pg_catalog", "a_very_long_routine_name" * 4, "text"
        )
        self.assertNotEqual(first, second)
        self.assertRegex(first, r"_[0-9a-f]{10}$")

        long_first = generate_openapi.unique_operation_id(
            "call", "pg_catalog", "a_very_long_routine_name_first", "first()"
        )
        long_second = generate_openapi.unique_operation_id(
            "call", "pg_catalog", "a_very_long_routine_name_second", "second()"
        )
        self.assertNotEqual(long_first, long_second)

    def test_human_text_replaces_underscores_only_in_prose(self):
        self.assertEqual(
            "Returns the current database name.",
            generate_openapi.human_text("Returns_the_current_database_name."),
        )

    def test_contract_rejects_underscores_in_description(self):
        spec = {"paths": {"/x": {"get": {"operationId": "exact_identifier"}}},
                "description": "not_natural_language"}
        with self.assertRaisesRegex(ValueError, "contain underscores"):
            generate_openapi.validate_spec_contract(spec)

    def test_build_spec_maps_routine_and_relation(self):
        metadata = {"server_version": "18.4", "server_version_num": 180004}
        routines = [
            {
                "schema_name": "pg_catalog",
                "routine_name": "abs",
                "routine_kind": "function",
                "identity_arguments": "integer",
                "result_type": "integer",
                "returns_set": False,
                "description": "absolute value",
                "arguments": [
                    {"name": "arg1", "mode": "in", "data_type": "integer", "has_default": False}
                ],
                "return_columns": [],
            }
        ]
        relations = [
            {
                "schema_name": "pg_catalog",
                "relation_name": "pg_tables",
                "relation_kind": "view",
                "description": "tables",
                "columns": [
                    {"name": "tablename", "data_type": "name", "nullable": True, "ordinal": 1}
                ],
            }
        ]

        spec = generate_openapi.build_spec("18", "postgres:18-alpine", metadata, routines, relations)

        self.assertEqual("3.1.0", spec["openapi"])
        self.assertEqual(2, len(spec["paths"]))
        self.assertIn("PostgresError", spec["components"]["schemas"])
        self.assertEqual(
            "password", spec["x-postgres-authentication"]["method"]
        )
        self.assertNotIn("securitySchemes", spec["components"])
        operation_ids = [
            operation["operationId"]
            for path_item in spec["paths"].values()
            for operation in path_item.values()
        ]
        self.assertEqual(len(operation_ids), len(set(operation_ids)))
        descriptions = [
            operation["description"]
            for path_item in spec["paths"].values()
            for operation in path_item.values()
        ]
        self.assertTrue(all("_" not in description for description in descriptions))


if __name__ == "__main__":
    unittest.main()
