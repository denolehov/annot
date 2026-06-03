# /// script
# requires-python = ">=3.11"
# dependencies = ["pillow"]
# ///
"""Derive the `annot (preview)` icon source from the stable icon.

The stable icon is a colored-gradient rounded square with a white asterisk.
Rotating the hue recolors the gradient while leaving the white mark untouched
(white has zero saturation), so preview windows are recognisable at a glance
without maintaining separate artwork.

The output is a single source PNG; feed it to `tauri icon` to generate the
full icon set (mirroring src-tauri/icons/):

    uv run scripts/make-preview-icon.py /tmp/preview-src.png
    pnpm tauri icon /tmp/preview-src.png --output src-tauri/icons-preview
"""
import sys
import tempfile
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "src-tauri" / "icons" / "icon.png"
DST = (
    Path(sys.argv[1])
    if len(sys.argv) > 1
    else Path(tempfile.gettempdir()) / "annot-preview-icon-source.png"
)

HUE_SHIFT_DEGREES = 120  # recolors green->blue, gold->magenta, purple->green

img = Image.open(SRC).convert("RGBA")
r, g, b, alpha = img.split()
hue, sat, val = Image.merge("RGB", (r, g, b)).convert("HSV").split()
shift = round(HUE_SHIFT_DEGREES / 360 * 256)
hue = hue.point(lambda x: (x + shift) % 256)
recolored = Image.merge("HSV", (hue, sat, val)).convert("RGB")
out = Image.merge("RGBA", (*recolored.split(), alpha))

DST.parent.mkdir(parents=True, exist_ok=True)
out.save(DST)
print(f"wrote {DST}")
