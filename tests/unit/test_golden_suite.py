import json
import unittest
from pathlib import Path

class TestGoldenSuite(unittest.TestCase):
    def setUp(self):
        self.root = Path(__file__).resolve().parent.parent.parent
        self.golden_path = self.root / "data/golden/golden_tasks.json"

    def test_golden_tasks_valid(self):
        self.assertTrue(self.golden_path.exists())
        with open(self.golden_path, "r", encoding="utf-8") as f:
            data = json.load(f)
        self.assertTrue(data.get("immutable"))
        tasks = data.get("tasks", [])
        self.assertGreaterEqual(len(tasks), 5)
        for t in tasks:
            self.assertIn("id", t)
            self.assertIn("category", t)
            self.assertIn("min_score", t)

if __name__ == "__main__":
    unittest.main()
