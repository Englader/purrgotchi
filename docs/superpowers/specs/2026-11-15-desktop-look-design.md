# Purrgotchi desktop look: design

## Intent

Paint the Claude desktop app's Code tab in Purrgotchi's Neon Grid style, the way
`DorDeorz/claude-neon-city-theme` does for its palette, plus a backdrop behind the
conversation. The terminal theme, HUD and band cat stay as they are.

## Goals

- Two colour variants, switchable: **Neon Grid** (near-black violet ground) and
  **Neon Grid Violet** (`#1a1633` ground). Same accents in both: magenta accent, ice
  text, cyan secondary text, mint success, red danger, yellow warning, violet "pro".
- A backdrop, switchable: none, a built-in original scene, or the person's own image.
- No effect on terminal sessions.

## Non-goals

- Light mode. The look is dark only (`only: 'dark'`).
- Restyling regular chat or permission prompts: the app does not let a mod paint them.
- Changing fonts or layout: the API has colours and a backdrop only.

## Anti-goals

- Shipping Cyberpunk 2077 / Edgerunners artwork, logos or fonts in the repository.
  They are CD Projekt's and Trigger's. The repo ships only original art; the person
  can point the mod at an image file of their own on their own machine.
- A backdrop that makes text hard to read.

## Approaches

1. **A new hook file in the existing `mod/`** (picked). `mod/hooks/look.ts` builds
   the colours with the engine's `make-colors.js` helper from the palette already in
   `mod/hooks/neon.ts`, and `register.ts` calls it at `session.start`. One plugin to
   install, one palette source.
2. A separate plugin/repository like the reference. Cleaner to share as "just a
   theme", but duplicates the palette and adds a second install. Not picked.
3. Hand-written design-system token JSON. The token names are not guessable and a
   wrong one silently paints nothing. Not picked.

## Design

- `mod/hooks/make-colors.js` + `.d.ts`: copied unchanged from the plugin-authoring
  skill (as its instructions direct).
- `mod/hooks/look.ts`: pure `lookFor(variant, backdrop)` returning `ThemeSetArgs`,
  and `startLook($, options)` called from the one `session.start` hook. Surfaces per
  variant (surface-0..3): Grid `#07050d #0d0a14 #141021 #1c1630`; Violet
  `#141029 #1a1633 #211c40 #29234d`. Both were run through `makeColors` and neither
  produces a "Desktop won't paint this theme" line.
- Options, as `userConfig` pickers in `plugin.json` (rows in the config menu):
  - `desktopLook`: `grid` (default) | `violet` | `off`
  - `desktopBackdrop`: `grid` (default) | `city` | `image` | `none`
- Commands: `/purr-look grid|violet|off` to switch in the running session
  (`off` calls `$.theme.clear()`).
- Backdrops:
  - `grid`: an SVG scene drawn by a `ui.render` hook on `Backdrop` (desktop and
    mobile only): perspective magenta grid, cyan horizon, low sun, and the pixel cat
    sitting on the horizon in a bottom corner, clear of the text column and prompt
    box. Slow CSS animation (grid drift, cat tail/blink), disabled under
    `prefers-reduced-motion`.
  - `city`: an original neon-city skyline scene in the same palette (inspired by the
    genre, not copied from any game or show).
  - `image`: `{ image: { source: 'backdrops/custom.jpg' } }` (or `.png`), a file the
    person drops into `mod/backdrops/`, which is gitignored. Limits from the API:
    JPEG/PNG, at most 1 MiB and 4096 px a side. The API allows no veil over a
    still image, so the picture shows as it is: a dark, low-contrast one reads
    best. Missing file falls back to `grid`.
- Tests beside the file (`look.test.ts`): each variant has no "won't paint" problem;
  option parsing; `off` sets nothing; scene SVG under the 131,072-character limit;
  hook passes on the terminal surface.
- README section: requirements (Claude Code 2.1.324+ in the Code tab), the options,
  the custom image folder, and how to remove the colours (Settings → code
  appearance → Remove colors).

## Left to the builder

- The exact scene art and animation timing.

## Not verifiable here

The desktop app cannot be run in the build environment. Tests cover shape and the
helper's checks; the final look needs a check in the real app, in a session started
after install.
