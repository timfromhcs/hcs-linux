#!/usr/bin/env python3
"""Generate the missing HCS app icons.

Kept as a script (not committed blobs) so the icon set stays internally
consistent: every icon shares the same frame, gradient and stroke language.
Run: python3 scripts/generate_icons.py
"""
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
OUT = REPO / "config" / "includes.chroot" / "usr" / "share" / "icons" / "hcs"

TEMPLATE = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128" width="128" height="128">
  <defs>
    <linearGradient id="bg" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#0f172a" />
      <stop offset="100%" stop-color="#05070a" />
    </linearGradient>
    <linearGradient id="fg" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#34d399" />
      <stop offset="100%" stop-color="#38bdf8" />
    </linearGradient>
  </defs>
  <rect x="8" y="8" width="112" height="112" rx="26" fill="url(#bg)" stroke="url(#fg)" stroke-width="2.5" />
{body}
</svg>
"""

STROKE = ('  <path d="{d}" fill="none" stroke="url(#fg)" stroke-width="5" '
          'stroke-linecap="round" stroke-linejoin="round" />\n')

FILL = ('  <circle cx="{cx}" cy="{cy}" r="{r}" fill="url(#fg)" opacity="0.85" />\n')

ICONS = {
    # Monitor: a screen with a stand.
    "hcs-monitor": [
        STROKE.format(d="M20 30 h88 v52 h-88 z"),
        STROKE.format(d="M64 82 v14"),
        STROKE.format(d="M46 96 h36"),
    ],
    # File manager: a folder with a tab.
    "hcs-fm": [STROKE.format(d="M20 34 h34 l10 12 h44 a6 6 0 0 1 6 6 v44 a6 6 0 0 1 -6 6 h-88 "
                              "a6 6 0 0 1 -6 -6 v-56 a6 6 0 0 1 6 -6 z")],
    # Terminal: prompt chevron plus a caret.
    "hcs-term": [STROKE.format(d="M28 38 l24 26 l-24 26"), STROKE.format(d="M60 90 h38")],
    # Screenshot: camera body plus lens.
    "hcs-shot": [
        STROKE.format(d="M22 48 h20 l8 -12 h28 l8 12 h20 v42 h-84 z"),
        STROKE.format(d="M64 50 a17 17 0 1 0 0.1 0"),
    ],
    # Notes: page with ruled lines.
    "hcs-notes": [
        STROKE.format(d="M32 26 h46 l18 18 v58 h-64 z"),
        STROKE.format(d="M44 58 h40"),
        STROKE.format(d="M44 72 h40"),
        STROKE.format(d="M44 86 h26"),
    ],
    # Settings: gear ring plus spokes.
    "hcs-settings": [
        STROKE.format(d="M64 46 a18 18 0 1 0 0.1 0"),
        STROKE.format(d="M64 34 v10"),
        STROKE.format(d="M64 84 v10"),
        STROKE.format(d="M34 64 h10"),
        STROKE.format(d="M84 64 h10"),
    ],
    # Update: circular arrow.
    "hcs-update": [
        STROKE.format(d="M64 34 a30 30 0 1 1 -28 18"),
        STROKE.format(d="M64 34 v24 h-24"),
    ],
    # RAG: layered documents plus a retrieval arc.
    "hcs-rag-ingest": [
        STROKE.format(d="M30 30 h44 l16 16 v56 h-60 z"),
        STROKE.format(d="M46 62 h32"),
        STROKE.format(d="M46 76 h32"),
        FILL.format(cx=96, cy=90, r=8),
    ],
    # QA agent: checkmark inside a rounded frame.
    "hcs-qa-agent": [
        STROKE.format(d="M30 34 h68 v60 h-68 z"),
        STROKE.format(d="M46 64 l12 12 l24 -26"),
    ],
}


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    for name, parts in ICONS.items():
        path = OUT / f"{name}.svg"
        path.write_text(TEMPLATE.format(body="".join(parts)), encoding="utf-8")
        print(f"[OK] {path.relative_to(REPO)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
