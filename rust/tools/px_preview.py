#!/usr/bin/env python3
"""Render `purrgotchi pixels sheet --json` output to a PNG (dev tool, not shipped).

usage: px_preview.py OUT.png [SHEET.json ...] [--scale N] [--only REGEX] [--cols N]

With no SHEET files, one sheet is read from stdin. Each sheet becomes a titled
section; cells are nearest-neighbour upscaled on the near-black violet ground
with their label underneath. --only keeps cells whose label matches REGEX.
"""
import json
import re
import sys

from PIL import Image, ImageDraw, ImageFont

GROUND = (13, 10, 20)  # #0d0a14
TEXT = (201, 195, 221)
TITLE = (10, 189, 198)


def font(size):
    try:
        return ImageFont.load_default(size=size)
    except TypeError:  # Pillow without FreeType sizing
        return ImageFont.load_default()


def main(argv):
    args, files, scale, only, cols = argv[1:], [], 10, None, None
    out = args.pop(0)
    while args:
        a = args.pop(0)
        if a == "--scale":
            scale = int(args.pop(0))
        elif a == "--only":
            only = re.compile(args.pop(0))
        elif a == "--cols":
            cols = int(args.pop(0))
        else:
            files.append(a)
    sheets = [json.load(open(f)) for f in files] or [json.load(sys.stdin)]
    for s in sheets:
        if only:
            s["cells"] = [c for c in s["cells"] if only.search(c["label"])]
        if cols:
            s["cols"] = cols

    allcells = [c for s in sheets for c in s["cells"]]
    cw = max(len(c["px"][0]) for c in allcells) + 2  # px, incl. 1 px margin each side
    ch = max(len(c["px"]) for c in allcells) + 2
    lab_f, title_f = font(max(11, scale + 2)), font(max(14, scale * 2))
    lab_h, title_h = max(11, scale + 2) + 8, max(14, scale * 2) + 12
    cell_w, cell_h = cw * scale, ch * scale + lab_h
    ncols = max(s["cols"] for s in sheets)
    rows = [(len(s["cells"]) + s["cols"] - 1) // s["cols"] for s in sheets]
    img = Image.new("RGB", (ncols * cell_w, sum(title_h + r * cell_h for r in rows)), GROUND)
    d = ImageDraw.Draw(img)
    y0 = 0
    for s, nrows in zip(sheets, rows):
        d.text((scale, y0 + 4), s.get("title", ""), fill=TITLE, font=title_f)
        y0 += title_h
        for i, c in enumerate(s["cells"]):
            ox = (i % s["cols"]) * cell_w
            oy = y0 + (i // s["cols"]) * cell_h
            px = c["px"]
            # bottom-align sprites so every cell shares one ground line
            top = oy + (ch - 1 - len(px)) * scale
            left = ox + (cw - len(px[0])) // 2 * scale
            for y, row in enumerate(px):
                for x, v in enumerate(row):
                    if v:
                        col = tuple(int(v[k:k + 2], 16) for k in (1, 3, 5))
                        d.rectangle([left + x * scale, top + y * scale, left + (x + 1) * scale - 1, top + (y + 1) * scale - 1], fill=col)
            d.text((ox + scale // 2, oy + ch * scale + 2), c["label"], fill=TEXT, font=lab_f)
        y0 += nrows * cell_h
    img.save(out)


if __name__ == "__main__":
    main(sys.argv)
