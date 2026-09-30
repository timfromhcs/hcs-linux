#!/usr/bin/env python3
"""
HCS Linux - Visual QA Verification and Image Entropy Auditor
Audits screenshots for dimensions, aspect ratio, file size, and visual entropy
to detect black screens, frozen framebuffers, or incomplete renders.
"""

import argparse
import json
import math
import sys
from pathlib import Path
from PIL import Image

def calculate_entropy(image: Image.Image) -> float:
    """Calculate the Shannon entropy of an image to detect black/blank screens."""
    histogram = image.histogram()
    total_pixels = sum(histogram)
    if total_pixels == 0:
        return 0.0

    entropy = 0.0
    for count in histogram:
        if count > 0:
            p = count / total_pixels
            entropy -= p * math.log2(p)
    return entropy

def audit_screenshot(image_path: Path):
    if not image_path.exists():
        return {
            "path": str(image_path),
            "status": "MISSING",
            "error": "File does not exist"
        }

    size_bytes = image_path.stat().st_size
    if size_bytes < 1000:
        return {
            "path": str(image_path),
            "size_bytes": size_bytes,
            "status": "FAIL",
            "error": "File size too small (likely truncated or failed capture)"
        }

    try:
        with Image.open(image_path) as img:
            width, height = img.size
            entropy = calculate_entropy(img)

            # A completely black or uniform screen has entropy near 0
            is_blank = entropy < 0.5

            status = "PASS" if not is_blank and width >= 720 and height >= 400 and size_bytes >= 1500 else "FAIL"

            return {
                "file": image_path.name,
                "width": width,
                "height": height,
                "size_bytes": size_bytes,
                "entropy": round(entropy, 3),
                "is_blank": is_blank,
                "status": status
            }
    except Exception as e:
        return {
            "file": image_path.name,
            "status": "ERROR",
            "error": str(e)
        }

def main():
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8")

    parser = argparse.ArgumentParser(description="Audit HCS visual QA screenshot artifacts")
    parser.add_argument("--dir", default=None, help="Directory containing screenshot PNGs")
    parser.add_argument("--file", default=None, help="Single screenshot PNG file to audit")
    parser.add_argument("--report", default="qa/reports/visual_entropy_audit.json", help="Path to save output audit report")
    args = parser.parse_args()

    repo_root = Path(__file__).resolve().parent.parent

    if args.file:
        single_file = Path(args.file)
        if not single_file.is_absolute():
            single_file = repo_root / single_file
        audit = audit_screenshot(single_file)
        status_sym = "[OK]" if audit.get("status") == "PASS" else "[FAIL]"
        entropy = audit.get("entropy", 0.0)
        size_kb = audit.get("size_bytes", 0) / 1024
        print(f"  {status_sym} {single_file.name:<28} | Entropy: {entropy:>5} | Size: {size_kb:>6.1f} KB | {audit.get('status')}")
        if audit.get("status") != "PASS":
            sys.exit(1)
        sys.exit(0)

    screenshots_dir = repo_root / (args.dir or "qa/screenshots")
    report_file = repo_root / args.report
    report_file.parent.mkdir(parents=True, exist_ok=True)

    print(f"=== HCS Visual QA Entropy and Artifact Audit ===")
    print(f"Auditing directory: {screenshots_dir}\n")

    if not screenshots_dir.exists():
        print(f"[ERROR] Directory not found: {screenshots_dir}", file=sys.stderr)
        sys.exit(1)

    png_files = sorted(screenshots_dir.glob("*.png"))
    if not png_files:
        print(f"[WARNING] No PNG screenshots found in {screenshots_dir}")
        sys.exit(1)

    results = []
    all_passed = True

    for png in png_files:
        audit = audit_screenshot(png)
        results.append(audit)
        status_sym = "[OK]" if audit.get("status") == "PASS" else "[FAIL]"
        entropy = audit.get("entropy", 0.0)
        size_kb = audit.get("size_bytes", 0) / 1024
        print(f"  {status_sym} {png.name:<28} | Entropy: {entropy:>5} | Size: {size_kb:>6.1f} KB | {audit.get('status')}")
        if audit.get("status") != "PASS":
            all_passed = False

    audit_summary = {
        "total_audited": len(results),
        "passed": sum(1 for r in results if r.get("status") == "PASS"),
        "failed": sum(1 for r in results if r.get("status") != "PASS"),
        "overall_status": "PASS" if all_passed else "FAIL",
        "results": results
    }

    with open(report_file, "w", encoding="utf-8") as f:
        json.dump(audit_summary, f, indent=2)

    print(f"\nAudit completed: {audit_summary['passed']}/{audit_summary['total_audited']} screenshots passed entropy verification.")
    print(f"Report saved to: {report_file}")

    if not all_passed:
        sys.exit(1)

if __name__ == "__main__":
    main()
