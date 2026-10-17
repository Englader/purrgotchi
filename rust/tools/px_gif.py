#!/usr/bin/env python3
"""Record the v4 demo GIF: real `purrgotchi statusline` frames (HUD box + pixel cat).

usage: px_gif.py BIN OUT.gif     (dev tool, not shipped; needs Pillow, jq-free)

Every frame renders against a fresh throwaway state dir under a temp HOME, so
nothing touches ~/.claude/purrgotchi. Sequence: idle life, ball scene, level-up
celebration, sleep. ANSI -> image: ▀ ▄ █ become pixels, text uses DejaVu Sans Mono.
"""
import json
import os
import re
import subprocess
import sys
import tempfile

from PIL import Image, ImageDraw, ImageFont

NOW0, CW, CH, COLS = 1800000000, 9, 18, 150
BG = (13, 10, 20)
FONT = "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"
TOKEN = re.compile(r"\x1b\[([0-9;]*)m|(.)", re.S)
BASE = {}
ADULT = {"level": 30, "total_tool_calls": 1500}


def render(bin_, now, patch, env_extra):
    with tempfile.TemporaryDirectory() as t:
        env = dict(os.environ, HOME=t, PURR_STATE_DIR=f"{t}/state", PURR_PIXELS="1", COLUMNS=str(COLS), PURR_NOW=str(now), **env_extra)
        payload = json.dumps({"workspace": {"current_dir": t}, "context_window": {"used_percentage": 20}})
        sp = f"{t}/state/state.json"
        if BASE:  # same cat every frame: hatch once, reuse that state
            os.makedirs(f"{t}/state")
            st = dict(BASE)
        else:
            subprocess.run([bin_, "statusline"], input=payload, text=True, env=dict(env, PURR_NOW=str(NOW0)), capture_output=True, check=True)
            st = json.load(open(sp))
            st.update(ADULT)
            BASE.update(st)
        st.update(patch)
        json.dump(st, open(sp, "w"))
        return subprocess.run([bin_, "statusline"], input=payload, text=True, env=env, capture_output=True, check=True).stdout


def draw_ansi(text, font):
    lines = text.rstrip("\n").split("\n")
    img = Image.new("RGB", (COLS * CW, len(lines) * CH), BG)
    d = ImageDraw.Draw(img)
    for r, line in enumerate(lines):
        fg, bg, dim, c = (201, 195, 221), None, False, 0
        for m in TOKEN.finditer(line):
            if m.group(2) is None:
                p = [int(x or 0) for x in m.group(1).split(";")]
                if p == [0]:
                    fg, bg, dim = (201, 195, 221), None, False
                elif p == [2]:
                    dim = True
                elif p[:2] == [38, 2]:
                    fg = tuple(p[2:5])
                elif p[:2] == [48, 2]:
                    bg = tuple(p[2:5])
                continue
            ch, x, y = m.group(2), c * CW, r * CH
            c += 1
            if bg:
                d.rectangle([x, y, x + CW - 1, y + CH - 1], fill=bg)
            if ch == "▀":
                d.rectangle([x, y, x + CW - 1, y + CH // 2 - 1], fill=fg)
            elif ch == "▄":
                d.rectangle([x, y + CH // 2, x + CW - 1, y + CH - 1], fill=fg)
            elif ch == "█":
                d.rectangle([x, y, x + CW - 1, y + CH - 1], fill=fg)
            elif ch != " ":
                col = tuple((a + b) // 2 for a, b in zip(fg, BG)) if dim else fg
                d.text((x, y), ch, fill=col, font=font)
    return img


def main(bin_, out):
    font = ImageFont.truetype(FONT, 15)
    frames = []

    def add(n, patch_of, env=None):
        for i in range(n):
            now = NOW0 + len(frames)
            frames.append(draw_ansi(render(bin_, now, patch_of(now, i), {"PURR_PLAYGROUND": "0", **(env or {})}), font))

    fed = lambda now: {"last_fed_at": now - 120}
    add(20, lambda now, i: fed(now))                                                  # calm idle life
    scene = lambda now, i: {**fed(now), "scene_kind": "BallChase", "scene_started_at": now - i}
    add(30, scene, {"PURR_PLAYGROUND": "1"})                                          # ball scene
    cel = lambda now, i: {**fed(now), "celebration_kind": "LevelUp", "levelup_renders_remaining": 2, "last_celebration_tick_at": now}
    add(6, cel)                                                                       # level up
    add(8, lambda now, i: {"last_fed_at": now - 30000})                               # sleep
    w, h = frames[0].size
    pal = [f.quantize(colors=48, dither=Image.Dither.NONE) for f in frames]
    pal[0].save(out, save_all=True, append_images=pal[1:], duration=[300] * len(pal), loop=0, optimize=True, disposal=1)
    print(len(frames), "frames", w, "x", h, os.path.getsize(out), "bytes")


if __name__ == "__main__":
    main(*sys.argv[1:3])
