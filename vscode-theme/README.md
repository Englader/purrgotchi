# Neon Grid

A Cyberpunk 2077-inspired colour theme for VS Code, designed to match the
[purrgotchi](https://github.com/Englader/purrgotchi) statusline. Neon cyan, yellow and
magenta accents on a dark canvas.

## Variants

- **Neon Grid**: pure black background.
- **Neon Grid Violet**: deep violet background (`#1a1633`).

## Switching

Press `Ctrl+K Ctrl+T` (`Cmd+K Cmd+T` on macOS) and pick a Neon Grid variant.

## Install from a .vsix

```
code --install-extension neon-grid-4.4.0.vsix
```

## Rebuilding

The theme files in `themes/` are generated from the sources in `src/`. After editing a source file:

```
python build.py
```

## Claude Code

The purrgotchi Claude Code mod ships matching "Neon Grid" and "Neon Grid Violet" themes, so your editor and terminal look the same.
