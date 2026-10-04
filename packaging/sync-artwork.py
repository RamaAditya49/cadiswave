#!/usr/bin/env python3
"""Update the installed SVG from the selected application PNG."""

import argparse
import base64
import struct
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--viewport", nargs=4, type=int, metavar=("X", "Y", "WIDTH", "HEIGHT"))
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    png = (root / "icons/cadiswave.png").read_bytes()
    if png[:8] != b"\x89PNG\r\n\x1a\n" or png[12:16] != b"IHDR":
        parser.error("The selected artwork must be a PNG file.")
    width, height = struct.unpack(">II", png[16:24])
    x, y, viewport_width, viewport_height = args.viewport or (0, 0, width, height)
    if min(x, y) < 0 or min(viewport_width, viewport_height) <= 0:
        parser.error("The viewport must have positive dimensions and nonnegative coordinates.")
    if x + viewport_width > width or y + viewport_height > height:
        parser.error("The viewport must remain inside the PNG.")
    data = base64.b64encode(png).decode("ascii")
    svg = (
        '<svg xmlns="http://www.w3.org/2000/svg" '
        'xmlns:xlink="http://www.w3.org/1999/xlink" width="128" height="128" '
        f'viewBox="{x} {y} {viewport_width} {viewport_height}">\n'
        f'  <image width="{width}" height="{height}" '
        f'xlink:href="data:image/png;base64,{data}"/>\n'
        '</svg>\n'
    )
    (root / "icons/cadiswave.svg").write_text(svg, encoding="utf-8")


if __name__ == "__main__":
    main()
