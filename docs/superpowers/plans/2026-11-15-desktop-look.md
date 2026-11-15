# Purrgotchi Desktop Look Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Paint the Claude desktop app's Code tab in Neon Grid or Neon Grid Violet, with a switchable backdrop (grid scene, city scene, the person's own image, or none).

**Architecture:** A pure module `mod/hooks/look.ts` turns two options into a `ThemeSetArgs` using the engine's `make-colors.js` helper and the palette already in `neon.ts`. `mod/hooks/scenes.ts` holds the two SVG scenes as strings. `mod/hooks/look-hooks.ts` wires them: set at `session.start`, a `Backdrop` render hook, and a `/purr-look` command. `register.ts` calls it.

**Tech Stack:** Claude Code 2.1.325 mods API (TypeScript, `claude plugin test`, `tsc`, `claude plugin validate`). No Rust changes.

**Spec:** `docs/superpowers/specs/2026-11-15-desktop-look-design.md`

## Global Constraints

- Dark only: `makeColors({ only: 'dark', ... })`.
- Surfaces (surface-0..3): Grid `#07050d #0d0a14 #141021 #1c1630`; Violet `#141029 #1a1633 #211c40 #29234d`.
- Text: primary `PALETTE.ice`, secondary `PALETTE.cyanLight`, muted `PALETTE.dimLight`.
- Roles: accent `magenta`, danger `red`, success `mint`, warning `yellow`, pro `violet`.
- Git: added `mint`, removed `red`, modified `yellow`, merged `violet`, closed `magenta`, conflicting `gold`, draft `dimLight`.
- No makeColors `problems` line may start `Desktop won't paint this theme`.
- `make-colors.js` / `make-colors.d.ts` are copied unchanged from the plugin-authoring skill folder.
- Only original art in the repo. No game or show artwork, logos, names or fonts.
- Scene SVG: under 131,072 characters, has `viewBox` and `preserveAspectRatio`, no `<script>`, no external URL, animation off under `prefers-reduced-motion`.
- The Backdrop hook draws only on `desktop` and `mobile`, and only for scenes this plugin set (`e.props.plugin`); otherwise `next(e)`.
- Existing terminal behaviour (band, terminal backdrop, Neon pieces) is untouched; their tests keep passing.
- Options (`userConfig` in `plugin.json`, string pickers): `desktopLook` = `grid` (default) | `violet` | `off`; `desktopBackdrop` = `grid` (default) | `city` | `image` | `none`.
- Custom image: `backdrops/custom.jpg`, else `backdrops/custom.png`, inside `mod/`; `mod/backdrops/` is gitignored except a `.gitkeep`. Glass over an image: `{ blur: 18, tint: <variant surface-0>, opacity: 0.72 }`.
- Gates: `tsc` over the mod, `claude plugin validate mod`, `claude plugin test mod`.
- Commits: no attribution lines.

## Review Focus

1. **Custom image missing or refused** (option `image`, no file, or a file over 1 MiB). The look still applies with the `grid` scene, never no theme. Owner: Task 3 (`image falls back to the grid scene`).
2. **A stored option outside the picker values** (old settings, hand edit). Treated as the default. Owner: Task 1 (`parse falls back to defaults`).
3. **`off` with a look already standing** (`/purr-look off`). The theme is cleared; `off` at session start sets and clears nothing, so another mod's theme survives. Owner: Task 3 (`off at start touches nothing`, `purr-look off clears`).
4. **Another plugin's backdrop scene.** Our hook passes it on untouched. Owner: Task 3 (`another plugin's scene passes`).
5. **`/purr-look` with junk or no argument.** Answers with the usage line and the current look; changes nothing. Owner: Task 3 (`purr-look with a bad argument changes nothing`).

## File Structure

| path | change | responsibility |
|---|---|---|
| `mod/hooks/make-colors.js`, `.d.ts` | create (copy) | the engine's colour helper, unchanged |
| `mod/hooks/look.ts` | create | option parsing, colours per variant, `lookFor` |
| `mod/hooks/scenes.ts` | create | the `grid` and `city` SVG strings |
| `mod/hooks/look-hooks.ts` | create | `startLook`, `registerLook` |
| `mod/hooks/*.test.ts` | create | one beside each of the three |
| `mod/hooks/register.ts` | modify | calls `startLook` and `registerLook` |
| `mod/.claude-plugin/plugin.json` | modify | `userConfig`, version 4.5.0, description |
| `mod/.gitignore`, `mod/backdrops/.gitkeep` | modify/create | the custom image folder |
| `README.md`, `CHANGELOG.md` | modify | docs |

---

### Task 1: Colours and options (`look.ts`)

**Files:** Create `mod/hooks/make-colors.js`, `mod/hooks/make-colors.d.ts`, `mod/hooks/look.ts`, `mod/hooks/look.test.ts`.

**Interfaces — Produces:**

```ts
export type LookVariant = 'grid' | 'violet'
export type LookChoice = LookVariant | 'off'
export type BackdropChoice = 'grid' | 'city' | 'image' | 'none'
export const SURFACES: Readonly<Record<LookVariant, readonly [string, string, string, string]>>
export const IMAGE_FILES: readonly ['backdrops/custom.jpg', 'backdrops/custom.png']
export function parseLook(v: unknown): LookChoice          // default 'grid'
export function parseBackdrop(v: unknown): BackdropChoice  // default 'grid'
export function colorsFor(variant: LookVariant): { theme: ThemeColors; problems: string[] }
/** `image` is the file to use when backdrop is 'image'. */
export function lookFor(variant: LookVariant, backdrop: BackdropChoice, image?: string): ThemeSetArgs
```

`lookFor` backdrops: `grid`/`city` → `{ scene: 'purr-grid' | 'purr-city', glass: false }`; `image` → `{ image: { source: image ?? IMAGE_FILES[0] }, glass: {...} }` per Global Constraints; `none` → no `backdrop` key.

- [ ] **Step 1:** Copy the two helper files from the skill folder.
- [ ] **Step 2:** Write `look.test.ts`:
  - `each variant paints`: for both variants, `colorsFor(v).problems` has no line starting `Desktop won't paint`, and `theme.inputs.dark['gray-900']` equals `SURFACES[v][0]`.
  - `parse falls back to defaults`: `parseLook('violet') === 'violet'`, `parseLook('off') === 'off'`, `parseLook('nope') === 'grid'`, `parseLook(undefined) === 'grid'`, `parseLook(3) === 'grid'`; same shape for `parseBackdrop` with `'city'`, `'image'`, `'none'`, junk → `'grid'`.
  - `lookFor shapes`: `none` has no `backdrop`; `grid` is `{ scene: 'purr-grid', glass: false }`; `city` is `purr-city`; `image` with `'backdrops/custom.png'` carries that source and glass tint `SURFACES[v][0]`.
- [ ] **Step 3:** Run `claude plugin test mod`; the new tests fail (module missing).
- [ ] **Step 4:** Implement `look.ts`.
- [ ] **Step 5:** Run the gates; all pass. Commit `feat(mod): desktop look colours and options`.

### Task 2: Scenes (`scenes.ts`)

**Files:** Create `mod/hooks/scenes.ts`, `mod/hooks/scenes.test.ts`.

**Interfaces — Consumes:** `LookVariant`, `SURFACES`, `PALETTE`. **Produces:**

```ts
export type SceneName = 'purr-grid' | 'purr-city'
export const SCENE_ALT: Readonly<Record<SceneName, string>>
export function isScene(name: string): name is SceneName
export function sceneSvg(name: SceneName, variant: LookVariant): string
```

Art, both in `PALETTE` colours over the variant's surface-0, `viewBox="0 0 1200 800"`, root `preserveAspectRatio="xMidYMax slice"`:

- `purr-grid`: purple sky glow, low yellow-to-magenta sun right of centre, cyan horizon line at y=470, magenta perspective floor grid whose horizontal lines drift toward the viewer (CSS), and a pixel cat (rects, cyan) sitting on the horizon in a nested `<svg>` pinned `xMaxYMax meet` so it stays in the right corner; tail flick and blink via CSS.
- `purr-city`: layered skyline silhouettes (three depths, darker to lighter violet) generated from a fixed seed, lit windows in cyan/yellow/magenta with a slow flicker on a few, a magenta haze at the horizon, and the same pinned cat on a rooftop.
- All motion sits inside `@media (prefers-reduced-motion: no-preference)`. Low contrast behind the text column: nothing brighter than 60% opacity above y=470.

- [ ] **Step 1:** Write `scenes.test.ts`: for each scene × variant, the string starts `<svg`, is under 131,072 characters, contains `viewBox="0 0 1200 800"`, `preserveAspectRatio="xMidYMax slice"`, `prefers-reduced-motion`, the variant's surface-0, and matches none of `/<script/i`, `/https?:/`, `/url\(\s*['"]?(?!#)/`; `isScene('purr-grid')` true, `isScene('lagoon')` false; the two variants differ.
- [ ] **Step 2:** Run tests; they fail.
- [ ] **Step 3:** Implement `scenes.ts`; render both scenes in a browser at 1280×820 and 420×900 and look at them.
- [ ] **Step 4:** Run the gates. Commit `feat(mod): grid and city backdrop scenes`.

### Task 3: Wiring (`look-hooks.ts`, manifest)

**Files:** Create `mod/hooks/look-hooks.ts`, `mod/hooks/look-hooks.test.ts`, `mod/backdrops/.gitkeep`. Modify `mod/hooks/register.ts`, `mod/.claude-plugin/plugin.json`, `mod/.gitignore`.

**Interfaces — Consumes:** everything Tasks 1–2 produce. **Produces:**

```ts
export async function startLook($: Pick<EngineInterface, 'theme' | 'command'>, options: PluginOptions): Promise<void>
export function registerLook(on: On, options: PluginOptions): void
```

Behaviour:

- `startLook` registers `/purr-look` (`description: 'Desktop look: grid, violet or off'`), then if the look is not `off` applies it. Applying: `$.theme.set(lookFor(...))`; for `image` try each of `IMAGE_FILES` in order and on a throw fall back to `lookFor(variant, 'grid')`. Never throws.
- `registerLook`: a `ui.render` hook on `{ component: 'Backdrop' }` that returns `Svg({ source: sceneSvg(scene, variant), alt: SCENE_ALT[scene] })` when the surface is `desktop` or `mobile`, `e.props.plugin === 'purrgotchi'` and `isScene(e.props.scene)`; else `next(e)`. A `command.run` hook on `{ command: 'purr-look' }`: `grid`/`violet` apply that variant with the configured backdrop and answer `Desktop look: <Neon Grid|Neon Grid Violet>.`; `off` calls `$.theme.clear()` and answers `Desktop look off.`; anything else answers `Usage: /purr-look grid|violet|off (now: <current>)` and changes nothing.
- The current variant lives in a module variable (a reload re-reads the options).
- `register.ts`: `register = (on, options)`; `startLook($, options)` inside the existing `session.start` hook after `startBackdrop`; `registerLook(on, options)` with the other registers.
- `plugin.json`: version `4.5.0`; description gains "and the Neon Grid desktop look"; `userConfig` with the two string fields, their `options` lists and defaults from Global Constraints.
- `mod/.gitignore` adds `backdrops/*` and `!backdrops/.gitkeep`.

- [ ] **Step 1:** Write `look-hooks.test.ts` (kit in `testkit.ts`; spy on `theme.set` / `theme.clear` from beneath with `on`):
  - `start applies the configured look`: options `{ desktopLook: 'violet', desktopBackdrop: 'city' }` → one `theme.set` whose `backdrop.scene` is `purr-city` and whose colours carry `#141029`.
  - `off at start touches nothing`: zero `theme.set`, zero `theme.clear`.
  - `image falls back to the grid scene`: `theme.set` throws for any `backdrop.image` → last successful set has `scene: 'purr-grid'`.
  - `image uses the png when the jpg is refused`.
  - `purr-look off clears`, `purr-look violet applies violet`, `purr-look with a bad argument changes nothing` (text starts `Usage:`).
  - `backdrop draws on desktop and mobile`, `backdrop passes on the terminal`, `another plugin's scene passes`, `an unknown scene passes`.
- [ ] **Step 2:** Run tests; they fail.
- [ ] **Step 3:** Implement and wire.
- [ ] **Step 4:** Run all three gates; every existing test still passes. Commit `feat(mod): desktop look wiring, options and /purr-look`.

### Task 4: Docs

**Files:** Modify `README.md`, `CHANGELOG.md`.

- [ ] **Step 1:** README section "Desktop look": what it paints and what it cannot (regular chat, permission prompts), requirement (Claude Code 2.1.324+ in the Code tab), the two options, `/purr-look`, the custom image folder with its limits (JPEG/PNG, ≤ 1 MiB, ≤ 4096 px a side, about 2560 wide suggested, your own file, not committed), and removal (Settings → code appearance → Remove colors).
- [ ] **Step 2:** CHANGELOG entry for 4.5.0.
- [ ] **Step 3:** Commit `docs: desktop look`, push.
