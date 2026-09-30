import unittest
from pathlib import Path
import yaml

class TestModelsLock(unittest.TestCase):
    def setUp(self):
        self.root = Path(__file__).resolve().parent.parent.parent
        self.lock_path = self.root / "vendor/locks/models.lock.yaml"

    def test_lockfile_exists(self):
        self.assertTrue(self.lock_path.exists(), "models.lock.yaml must exist")

    def test_all_models_have_sha256_and_license(self):
        with open(self.lock_path, "r", encoding="utf-8") as f:
            data = yaml.safe_load(f)
        models = data.get("models", [])
        self.assertGreaterEqual(len(models), 5)
        for m in models:
            self.assertIn("id", m)
            self.assertIn("sha256", m)
            self.assertEqual(len(m["sha256"]), 64)
            self.assertIn("license", m)
            self.assertNotIn(m.get("revision"), ["main", "master", "latest"])

if __name__ == "__main__":
    unittest.main()
