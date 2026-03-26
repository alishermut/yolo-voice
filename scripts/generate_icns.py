#!/usr/bin/env python3
"""Generate a macOS .icns file from a source PNG.

Packs the source image at multiple resolutions into Apple's ICNS container format.
ICNS format: 4-byte magic ('icns') + 4-byte total size, then entries:
  Each entry: 4-byte type + 4-byte size (incl. header) + PNG data

Requires: Pillow (`pip install Pillow`)
Usage:    python scripts/generate_icns.py
"""

import io
import struct
import sys
from pathlib import Path

try:
    from PIL import Image
except ImportError:
    print("Error: Pillow is required. Install with: pip install Pillow")
    sys.exit(1)

# ICNS icon types that accept PNG data (macOS 10.7+)
ICNS_TYPES = [
    (b"icp4", 16),    # 16x16
    (b"icp5", 32),    # 32x32
    (b"icp6", 64),    # 64x64
    (b"ic07", 128),   # 128x128
    (b"ic08", 256),   # 256x256
    (b"ic09", 512),   # 512x512
    (b"ic10", 1024),  # 1024x1024 (512@2x)
]

ICNS_MAGIC = b"icns"

def png_bytes(img: Image.Image, size: int) -> bytes:
    """Resize image and return PNG bytes."""
    resized = img.resize((size, size), Image.LANCZOS)
    buf = io.BytesIO()
    resized.save(buf, format="PNG")
    return buf.getvalue()

def build_icns(source_path: Path) -> bytes:
    """Build ICNS binary from a source PNG."""
    img = Image.open(source_path).convert("RGBA")

    entries = []
    for icon_type, size in ICNS_TYPES:
        # Skip sizes larger than source
        if size > max(img.width, img.height) * 2:
            continue
        data = png_bytes(img, size)
        # Entry: type(4) + size(4) + png_data
        entry_size = 8 + len(data)
        entry = icon_type + struct.pack(">I", entry_size) + data
        entries.append(entry)

    # File: magic(4) + total_size(4) + entries
    body = b"".join(entries)
    total_size = 8 + len(body)
    return ICNS_MAGIC + struct.pack(">I", total_size) + body

def main():
    project_root = Path(__file__).resolve().parent.parent
    source = project_root / "src-tauri" / "icons" / "icon.png"
    output = project_root / "src-tauri" / "icons" / "icon.icns"

    if not source.exists():
        print(f"Error: Source icon not found at {source}")
        sys.exit(1)

    icns_data = build_icns(source)
    output.write_bytes(icns_data)
    print(f"Generated {output} ({len(icns_data):,} bytes)")

if __name__ == "__main__":
    main()
