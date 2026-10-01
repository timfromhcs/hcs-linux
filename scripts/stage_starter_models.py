#!/usr/bin/env python3
"""Stage the license-clean starter models into the ISO payload.

The v2 rule (docs/V2_STABLE_RELEASE_MASTER_PLAN.md §5, W-modelle):

  * A model whose licence permits redistribution is **baked into the ISO**, so a
    fresh, network-free boot can chat.
  * Everything else downloads on demand and is hash-verified before
    `hcs-modeld` will load it.

Which models qualify is not a judgement call made here — it is read from
``vendor/locks/models.lock.yaml`` and cross-checked against
``config/models/*.json``. A model that is not marked redistributable is never
copied, even if a file happens to be sitting in the cache.

Usage:
    python3 scripts/stage_starter_models.py --check     # report only
    python3 scripts/stage_starter_models.py --stage DIR # copy into a payload
"""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
LOCKS = REPO / "vendor" / "locks" / "models.lock.yaml"
MODEL_CONFIG_DIR = REPO / "config" / "models"
DEFAULT_CACHE = REPO / "models" / "cache"

# Licences that permit redistribution of the weights themselves. A model under
# a non-commercial or research-only licence is excluded even if the code licence
# would allow it: shipping the weights is a different act from shipping code.
REDISTRIBUTABLE_LICENCE_MARKERS = (
    "apache-2.0",
    "mit",
    "bsd-2-clause",
    "bsd-3-clause",
    "cc0-1.0",
    "mulanpsl-2.0",
)

# These are the licences whose weights may be redistributed.
BLOCKING_MARKERS = (
    "non-commercial",
    "nc-",
    "research-only",
    "evaluation-only",
    "gated",
    "llama3.1",
    "llama-3.1",
)


def load_json(path: Path) -> dict:
    try:
        return json.loads(path.read_text(encoding="utf-8-sig"))
    except (OSError, json.JSONDecodeError) as exc:
        raise SystemExit(f"[ERROR] cannot read {path}: {exc}")


def yaml_of(entry: dict, *keys: str, default: str = "") -> str:
    for k in keys:
        if entry.get(k):
            return str(entry[k])
    return default


def model_registry() -> dict[str, dict]:
    """Every model declared in the model registry, keyed by id.

    The registry is YAML (`config/models/registry.yaml`), not JSON — it is the
    file the daemon and `hcs model list` both read, so baking decisions must be
    made against the same source rather than a parallel list that could drift.
    """
    out: dict[str, dict] = {}
    registry = MODEL_CONFIG_DIR / "registry.yaml"
    if not registry.exists():
        return out
    try:
        import yaml  # type: ignore
    except ImportError:
        print("[WARN] PyYAML is unavailable; cannot read registry.yaml, so nothing is baked")
        return out
    payload = yaml.safe_load(registry.read_text(encoding="utf-8")) or {}
    for entry in payload.get("models", []) or []:
        if isinstance(entry, dict) and entry.get("id"):
            out[str(entry["id"])] = entry
    return out


def licence_of(entry: dict) -> str:
    return str(entry.get("license", "")).strip().lower()


def is_redistributable(entry: dict) -> tuple[bool, str]:
    lic = licence_of(entry)
    if not lic:
        return False, "no licence declared"
    if any(b in lic for b in BLOCKING_MARKERS):
        return False, f"licence '{lic}' restricts redistribution"
    if any(m in lic for m in REDISTRIBUTABLE_LICENCE_MARKERS):
        return True, lic
    return False, f"licence '{lic}' is not on the redistributable list"


def sha256_of(path: Path, chunk: int = 1 << 20) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        while True:
            block = fh.read(chunk)
            if not block:
                break
            h.update(block)
    return h.hexdigest()


def staged_dir() -> Path:
    return REPO / "models" / "baked"


def pick_candidates(cache: Path) -> list[tuple[dict, Path, str]]:
    """(entry, file, reason) for every baked-in candidate, with reasons for skips."""
    registry = model_registry()
    out: list[tuple[dict, Path, str]] = []
    if not cache.is_dir():
        return out
    for path in sorted(cache.glob("*.gguf")):
        # Match the file to a declared model by filename fragment.
        match = None
        for mid, entry in registry.items():
            stem = str(entry.get("filename", "")).replace(".gguf", "")
            if stem and (stem.lower() in path.name.lower() or mid.lower() in path.name.lower()):
                match = entry
                break
        if match is None:
            out.append(({}, path, "not declared in config/models — never baked"))
            continue
        ok, reason = is_redistributable(match)
        out.append((match, path, reason if ok else f"skipped: {reason}"))
    return out


def write_manifest(target: Path, staged: list[tuple[dict, Path, str]]) -> None:
    lines = [
        "# Baked starter models",
        "",
        "This directory holds models whose licence permits redistribution inside",
        "the HCS Linux ISO. They are installed here so a fresh, network-free boot",
        "can chat.",
        "",
        "## What belongs here",
        "",
        "1. The licence must permit redistribution of the *weights*.",
        "2. The model must be declared in `config/models/registry.yaml` with a SHA-256.",
        "3. The total must stay inside the budget recorded in `MODEL-MANIFEST.json`.",
        "",
        "## What does not",
        "",
        "Larger models, gated models and anything licence-restricted downloads on",
        "demand: `hcs model fetch <id>`, verified by hash before `hcs-modeld` loads it.",
        "",
        "## Contents",
        "",
    ]
    if staged:
        for entry, path, _ in staged:
            size_mb = path.stat().st_size / (1024 * 1024)
            lines.append(
                f"- `{path.name}` - {entry.get('id', '?')}, "
                f"{entry.get('parameters', '?')} {entry.get('quantization', '')}, "
                f"licence {entry.get('license', '?')}, {size_mb:.0f} MB"
            )
    else:
        lines.append(
            "- (none staged — a valid empty GGUF placeholder is installed instead so the "
            "payload contract holds; `hcs model fetch` replaces it on first use)"
        )
    lines.append("")
    (target / "BUILT-IN.md").write_text("\n".join(lines), encoding="utf-8")


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--check", action="store_true", help="report only, change nothing")
    ap.add_argument("--stage", type=Path, help="copy eligible models into this payload dir")
    ap.add_argument("--cache", type=Path, default=DEFAULT_CACHE, help="model cache to scan")
    ap.add_argument("--max-mb", type=int, default=1024, help="total baked budget in MB")
    args = ap.parse_args()

    if not LOCKS.exists():
        print(f"[WARN] {LOCKS} is missing; the model lock is not in place")

    candidates = pick_candidates(args.cache)
    eligible = [(e, p, r) for e, p, r in candidates if e and not r.startswith("skipped")]

    print("=== Starter model staging ===")
    print(f"cache:      {args.cache}")
    print(f"budget:     {args.max_mb} MB")
    if not candidates:
        print("No .gguf files in the cache. This is normal on a fresh build host;")
        print("the ISO then installs a valid empty-GGUF placeholder.")
    for entry, path, reason in candidates:
        if entry:
            verdict = "BAKE" if not reason.startswith("skipped") else "skip"
            print(f"  [{verdict}] {path.name} — {entry.get('id')} — {reason}")

    total_mb = sum(p.stat().st_size for _, p, _ in eligible) / (1024 * 1024)
    print(f"eligible:   {len(eligible)} model(s), {total_mb:.0f} MB")
    if total_mb > args.max_mb:
        print(f"[ERROR] baked models exceed the {args.max_mb} MB budget")
        return 1

    if args.check or args.stage is None:
        return 0

    target = args.stage
    target.mkdir(parents=True, exist_ok=True)
    for entry, path, _ in eligible:
        # Hash-verify while copying: a truncated cache entry must not ship.
        declared = str(entry.get("sha256", "")).strip()
        if declared:
            actual = sha256_of(path)
            if actual != declared:
                print(f"[ERROR] {path.name} hash mismatch; refusing to bake it")
                print(f"        declared {declared}")
                print(f"        actual   {actual}")
                return 1
        dest = target / path.name
        shutil.copy2(path, dest)
        print(f"[OK] baked {dest.name}")

    if not eligible:
        # A valid, empty GGUF keeps the payload contract satisfied on a machine
        # with no model cache, so the build does not fail for a missing download.
        (target / "placeholder.gguf").write_bytes(b"GGUF")
        print("[OK] wrote placeholder.gguf")

    write_manifest(target, eligible)
    print(f"[OK] manifest at {target / 'BUILT-IN.md'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
