#!/usr/bin/env python3
"""Build the Neon Grid VS Code theme JSON files from the JSONC sources in src/.

Usage: python build.py
"""
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))

THEMES = [
    ("src/neon-grid.jsonc", "themes/neon-grid-color-theme.json", "Neon Grid"),
    ("src/neon-grid-violet.jsonc", "themes/neon-grid-violet-color-theme.json", "Neon Grid Violet"),
]


def strip_jsonc(text):
    """Remove // and /* */ comments and trailing commas, leaving strings intact."""
    out = []
    i, n = 0, len(text)
    while i < n:
        c = text[i]
        if c == '"':
            j = i + 1
            while j < n:
                if text[j] == "\\":
                    j += 2
                    continue
                if text[j] == '"':
                    break
                j += 1
            out.append(text[i:j + 1])
            i = j + 1
        elif text.startswith("//", i):
            while i < n and text[i] != "\n":
                i += 1
        elif text.startswith("/*", i):
            end = text.find("*/", i + 2)
            i = n if end == -1 else end + 2
        else:
            out.append(c)
            i += 1
    cleaned = "".join(out)

    # Second pass: drop trailing commas (outside strings).
    res = []
    i, n = 0, len(cleaned)
    while i < n:
        c = cleaned[i]
        if c == '"':
            j = i + 1
            while j < n:
                if cleaned[j] == "\\":
                    j += 2
                    continue
                if cleaned[j] == '"':
                    break
                j += 1
            res.append(cleaned[i:j + 1])
            i = j + 1
        elif c == ",":
            k = i + 1
            while k < n and cleaned[k].isspace():
                k += 1
            if k < n and cleaned[k] in "]}":
                i += 1
            else:
                res.append(c)
                i += 1
        else:
            res.append(c)
            i += 1
    return "".join(res)


def load_source(path):
    with open(path, encoding="utf-8") as f:
        text = strip_jsonc(f.read()).strip()
    # Sources are a settings.json fragment: top-level keys without outer braces.
    if not text.startswith("{"):
        text = "{" + text.rstrip().rstrip(",") + "}"
    return json.loads(text)


def build(src, dst, name):
    data = load_source(os.path.join(HERE, src))
    colors = data.get("workbench.colorCustomizations", {})
    # Drop scoped entries such as "[Theme Name]": {...}; they only make sense in settings.json.
    colors = {k: v for k, v in colors.items() if not k.startswith("[")}
    token = data.get("editor.tokenColorCustomizations", {})
    semantic = data.get("editor.semanticTokenColorCustomizations", {})
    theme = {
        "$schema": "vscode://schemas/color-theme",
        "name": name,
        "type": "dark",
        "semanticHighlighting": True,
        "colors": colors,
        "tokenColors": token.get("textMateRules", []),
        "semanticTokenColors": semantic.get("rules", {}),
    }
    out = os.path.join(HERE, dst)
    os.makedirs(os.path.dirname(out), exist_ok=True)
    with open(out, "w", encoding="utf-8", newline="\n") as f:
        json.dump(theme, f, indent=2, ensure_ascii=False)
        f.write("\n")
    print("wrote %s (%d colors, %d tokenColors, %d semantic)" % (
        dst, len(theme["colors"]), len(theme["tokenColors"]), len(theme["semanticTokenColors"])))


if __name__ == "__main__":
    for s, d, n in THEMES:
        build(s, d, n)
