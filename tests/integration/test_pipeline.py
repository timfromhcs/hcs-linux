import json
import subprocess
import sys
import unittest
from pathlib import Path

class TestSystemIntegration(unittest.TestCase):
    def setUp(self):
        self.root = Path(__file__).resolve().parent.parent.parent

    def test_security_audit_clean(self):
        cmd = [sys.executable, str(self.root / "scripts/run_security_audit.py")]
        res = subprocess.run(cmd, capture_output=True, text=True)
        self.assertEqual(res.returncode, 0, f"Security audit failed: {res.stderr}")

    def test_stress_suite_passing(self):
        cmd = [sys.executable, str(self.root / "scripts/run_stress_test.py")]
        res = subprocess.run(cmd, capture_output=True, text=True)
        self.assertEqual(res.returncode, 0, f"Stress suite failed: {res.stderr}")

    def test_sbom_generation(self):
        cmd = [sys.executable, str(self.root / "scripts/generate_sbom.py")]
        res = subprocess.run(cmd, capture_output=True, text=True)
        self.assertEqual(res.returncode, 0, f"SBOM generation failed: {res.stderr}")
        sbom_file = self.root / "dist/SBOM.spdx.json"
        self.assertTrue(sbom_file.exists())
        with open(sbom_file, "r", encoding="utf-8") as f:
            data = json.load(f)
        self.assertEqual(data.get("spdxVersion"), "SPDX-2.3")

if __name__ == "__main__":
    unittest.main()
