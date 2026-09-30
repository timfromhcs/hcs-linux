#!/usr/bin/env python3
"""
HCS Linux Artwork & Visual Assets Generator
Renders high-resolution neural glass wallpapers, boot splash, and application icons.
Adheres to GEMINI.md Sections 36-40, 150-155.
"""

import math
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont, ImageFilter

def generate_wallpaper(output_path: Path):
    width, height = 1920, 1080
    im = Image.new("RGBA", (width, height), (13, 17, 23, 255))
    draw = ImageDraw.Draw(im)

    # Base radial glow
    cx, cy = width // 2, height // 2
    for r in range(600, 50, -25):
        alpha = int(18 * (1.0 - r / 600.0))
        draw.ellipse([cx - r, cy - r, cx + r, cy + r], fill=(56, 189, 248, alpha))

    # Second indigo ambient glow
    cx2, cy2 = int(width * 0.7), int(height * 0.4)
    for r in range(450, 40, -20):
        alpha = int(22 * (1.0 - r / 450.0))
        draw.ellipse([cx2 - r, cy2 - r, cx2 + r, cy2 + r], fill=(129, 140, 248, alpha))

    # Neural Synaptic Mesh
    nodes = []
    num_nodes = 36
    for i in range(num_nodes):
        angle = (2 * math.pi * i) / num_nodes
        dist = 220 + (i % 5) * 65 + math.sin(i * 1.5) * 45
        nx = int(cx + dist * math.cos(angle))
        ny = int(cy + (dist * 0.6) * math.sin(angle))
        nodes.append((nx, ny))

    # Interconnect nodes
    for i in range(len(nodes)):
        for j in range(i + 1, min(i + 6, len(nodes))):
            x1, y1 = nodes[i]
            x2, y2 = nodes[j]
            d = math.hypot(x2 - x1, y2 - y1)
            if d < 320:
                alpha = int(140 * (1.0 - d / 320.0))
                draw.line([(x1, y1), (x2, y2)], fill=(56, 189, 248, alpha), width=2)

    # Draw nodes
    for i, (nx, ny) in enumerate(nodes):
        node_col = (56, 189, 248, 220) if i % 2 == 0 else (129, 140, 248, 220)
        draw.ellipse([nx - 5, ny - 5, nx + 5, ny + 5], fill=node_col)
        draw.ellipse([nx - 2, ny - 2, nx + 2, ny + 2], fill=(248, 250, 252, 255))

    # Central Core Emblem Glow
    draw.ellipse([cx - 40, cy - 40, cx + 40, cy + 40], fill=(56, 189, 248, 40))
    draw.ellipse([cx - 24, cy - 24, cx + 24, cy + 24], fill=(56, 189, 248, 120), outline=(248, 250, 252, 240), width=3)
    draw.ellipse([cx - 10, cy - 10, cx + 10, cy + 10], fill=(248, 250, 252, 255))

    # Minimal brand stamp in bottom-right
    stamp_x, stamp_y = width - 240, height - 70
    draw.text((stamp_x, stamp_y), "HCS LINUX", fill=(56, 189, 248, 180))
    draw.text((stamp_x, stamp_y + 20), "Local-First AI OS", fill=(148, 163, 184, 140))

    output_path.parent.mkdir(parents=True, exist_ok=True)
    rgb_im = im.convert("RGB")
    rgb_im.save(output_path, "PNG", optimize=True)
    print(f"[OK] Generated Wallpaper: {output_path} ({width}x{height})")

def generate_grub_splash(output_path: Path):
    width, height = 1024, 768
    im = Image.new("RGB", (width, height), (15, 23, 42))
    draw = ImageDraw.Draw(im)

    # Top brand bar
    draw.rectangle([0, 0, width, 80], fill=(22, 27, 34))
    draw.line([(0, 80), (width, 80)], fill=(56, 189, 248), width=2)

    # Title
    draw.text((48, 24), "HCS LINUX 1.0.1", fill=(56, 189, 248))
    draw.text((48, 48), "Neural Glass | Wayland Cognitive Desktop", fill=(148, 163, 184))

    # Boot Menu Card Frame
    card_x1, card_y1 = 120, 160
    card_x2, card_y2 = width - 120, height - 160
    draw.rectangle([card_x1, card_y1, card_x2, card_y2], fill=(22, 27, 34), outline=(56, 189, 248), width=2)

    # Menu Area Hint
    draw.text((card_x1 + 32, card_y1 + 24), "Select Boot Option with Arrow Keys and press Enter:", fill=(248, 250, 252))

    # Subtle bottom bar
    draw.rectangle([0, height - 48, width, height], fill=(22, 27, 34))
    draw.text((48, height - 34), "Local AI Runtime: Qwen3-0.6B Resident | RAM Budget: <= 8 GB Peak", fill=(100, 116, 139))

    output_path.parent.mkdir(parents=True, exist_ok=True)
    im.save(output_path, "PNG")
    print(f"[OK] Generated GRUB Splash: {output_path} ({width}x{height})")

def generate_icon_png(name: str, color_main: tuple, color_sec: tuple, symbol: str, output_path: Path):
    size = 128
    im = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(im)

    # Rounded card background
    draw.rounded_rectangle([8, 8, size - 8, size - 8], radius=26, fill=color_main, outline=color_sec, width=3)

    # Draw centered symbol / badge
    cx, cy = size // 2, size // 2
    if symbol == "chat":
        draw.ellipse([cx - 22, cy - 22, cx + 22, cy + 22], fill=color_sec)
        draw.ellipse([cx - 10, cy - 10, cx + 10, cy + 10], fill=(248, 250, 252, 255))
        draw.polygon([(cx - 10, cy + 14), (cx - 24, cy + 32), (cx + 2, cy + 20)], fill=color_sec)
    elif symbol == "search":
        draw.ellipse([cx - 16, cy - 16, cx + 14, cy + 14], outline=color_sec, width=6)
        draw.line([(cx + 10, cy + 10), (cx + 28, cy + 28)], fill=color_sec, width=8)
    elif symbol == "control":
        draw.line([(cx - 26, cy - 16), (cx + 26, cy - 16)], fill=(51, 65, 85, 255), width=6)
        draw.ellipse([cx - 10, cy - 24, cx + 6, cy - 8], fill=(248, 250, 252, 255), outline=color_sec, width=3)
        draw.line([(cx - 26, cy + 16), (cx + 26, cy + 16)], fill=(51, 65, 85, 255), width=6)
        draw.ellipse([cx + 2, cy + 8, cx + 18, cy + 24], fill=(248, 250, 252, 255), outline=color_sec, width=3)
    elif symbol == "security":
        draw.polygon([(cx, cy - 26), (cx + 24, cy - 14), (cx + 24, cy + 12), (cx, cy + 28), (cx - 24, cy + 12), (cx - 24, cy - 14)], fill=color_sec)
        draw.ellipse([cx - 6, cy - 6, cx + 6, cy + 6], fill=(248, 250, 252, 255))
    elif symbol == "updater":
        draw.arc([cx - 22, cy - 22, cx + 22, cy + 22], start=45, end=315, fill=color_sec, width=6)
        draw.polygon([(cx + 12, cy - 24), (cx + 26, cy - 16), (cx + 16, cy - 6)], fill=color_sec)
    else:  # terminal
        draw.line([(cx - 18, cy - 14), (cx - 4, cy)], fill=color_sec, width=5)
        draw.line([(cx - 4, cy), (cx - 18, cy + 14)], fill=color_sec, width=5)
        draw.line([(cx + 2, cy + 14), (cx + 20, cy + 14)], fill=(52, 211, 153, 255), width=5)

    output_path.parent.mkdir(parents=True, exist_ok=True)
    im.save(output_path, "PNG")

def main():
    repo_root = Path(__file__).resolve().parent.parent
    wallpaper_path = repo_root / "assets/wallpapers/neural_glass_dark.png"
    grub_path = repo_root / "assets/boot/grub_splash.png"
    icons_dir = repo_root / "assets/icons/png"

    generate_wallpaper(wallpaper_path)
    generate_grub_splash(grub_path)

    icons = [
        ("hcs-chat", (30, 27, 75, 255), (129, 140, 248, 255), "chat"),
        ("hcs-search", (6, 78, 59, 255), (52, 211, 153, 255), "search"),
        ("hcs-control", (30, 41, 59, 255), (56, 189, 248, 255), "control"),
        ("hcs-security", (49, 16, 66, 255), (192, 132, 252, 255), "security"),
        ("hcs-updater", (12, 74, 110, 255), (56, 189, 248, 255), "updater"),
        ("hcs-terminal", (24, 24, 27, 255), (56, 189, 248, 255), "terminal"),
    ]

    for name, col1, col2, sym in icons:
        out = icons_dir / f"{name}.png"
        generate_icon_png(name, col1, col2, sym, out)
        print(f"[OK] Generated Icon PNG: {out}")

if __name__ == "__main__":
    main()
