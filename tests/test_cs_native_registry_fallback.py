"""A registry outage must never silently remove previously shipped native mods."""
import importlib.util
import io
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

path = Path(__file__).resolve().parents[1] / "tools/cs_native_mods.py"
spec = importlib.util.spec_from_file_location("cs_native_mods", path)
mod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)

class FallbackTests(unittest.TestCase):
    def run_fallback(self, previous, enabled="1", stale=False):
        release = {"assets": [{"name": "cs-native-mods.json", "browser_download_url":
                    "https://github.com/owner/repo/releases/download/latest/cs-native-mods.json"}]}
        def response(data):
            return io.BytesIO(json.dumps(data).encode())
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "mods"
            if stale:
                out.mkdir()
                (out / "old").mkdir()
            with patch.dict(os.environ, {"CS_EMPTY_REGISTRY_FALLBACK": enabled,
                                         "GITHUB_REPOSITORY": "owner/repo"}), \
                 patch.object(mod.urllib.request, "urlopen", side_effect=[response(release), response(previous)]):
                result = mod.missing_registry_fallback(["zoom", "minimap"], out)
            manifest = out.parent / "cs-native-mods.json"
            return result, json.loads(manifest.read_text()) if manifest.exists() else None

    def test_previous_empty_build_is_recorded(self):
        result, manifest = self.run_fallback({"built": [], "skipped": []})
        self.assertTrue(result)
        self.assertEqual(manifest["built"], [])
        self.assertEqual([item["id"] for item in manifest["skipped"]], ["zoom", "minimap"])
        self.assertTrue(all("404" in item["reason"] for item in manifest["skipped"]))

    def test_prior_mods_invalid_metadata_or_stale_bundles_fail_closed(self):
        for previous in ({"built": [{"id": "zoom"}], "skipped": []}, {}, {"built": [], "skipped": None}):
            self.assertFalse(self.run_fallback(previous)[0])
        self.assertFalse(self.run_fallback({"built": [], "skipped": []}, enabled="0")[0])
        self.assertFalse(self.run_fallback({"built": [], "skipped": []}, stale=True)[0])

if __name__ == "__main__":
    unittest.main()
