import importlib.util
import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parent.parent
SPEC = importlib.util.spec_from_file_location(
    "version_manifest", ROOT / "tools" / "version_manifest.py"
)
version_manifest = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(version_manifest)


class VersionManifestTests(unittest.TestCase):
    def test_build_manifest_includes_stable_and_prerelease_lines(self):
        upstream = json.loads(
            (ROOT / "tests" / "fixtures" / "upstream-versions.json").read_text()
        )

        manifest = version_manifest.build_manifest(upstream, "2026-07-21T00:00:00Z")

        self.assertEqual(["19beta2", "18", "17"], [v["id"] for v in manifest["versions"]])
        self.assertEqual("postgres:19beta2-alpine", manifest["versions"][0]["image"])
        self.assertTrue(manifest["versions"][0]["prerelease"])
        self.assertFalse(manifest["versions"][1]["prerelease"])

    def test_resolve_accepts_id_major_release_and_image_tag(self):
        manifest = {
            "versions": [
                {
                    "id": "18",
                    "major": 18,
                    "release": "18.4",
                    "image": "postgres:18-alpine",
                    "prerelease": False,
                }
            ]
        }

        for value in ("18", "18.4", "postgres:18-alpine"):
            with self.subTest(value=value):
                self.assertEqual("18", version_manifest.resolve(manifest, value)["id"])


if __name__ == "__main__":
    unittest.main()
