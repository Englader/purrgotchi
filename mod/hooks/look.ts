// The desktop look: Neon Grid colours for the Claude desktop app's Code tab, and the backdrop choice. Pure: no `$`.
import type { ThemeColors, ThemeSetArgs } from 'claude-code'

import { makeColors } from './make-colors.js'
import { PALETTE } from './neon'

export type LookVariant = 'grid' | 'violet'
export type LookChoice = LookVariant | 'off'
export type BackdropChoice = 'grid' | 'city' | 'image' | 'none'

export const LOOK_NAMES: Readonly<Record<LookVariant, string>> = { grid: 'Neon Grid', violet: 'Neon Grid Violet' }

/** surface-0 (the page) to surface-3 (menus), darkest first. */
export const SURFACES: Readonly<Record<LookVariant, readonly [string, string, string, string]>> = {
  grid: ['#07050d', '#0d0a14', '#141021', '#1c1630'],
  violet: ['#141029', '#1a1633', '#211c40', '#29234d'],
}

/** The person's own picture, in the mod's folder; the first that the app takes is used. */
export const IMAGE_FILES = ['backdrops/custom.jpg', 'backdrops/custom.png'] as const

const LOOKS: readonly LookChoice[] = ['grid', 'violet', 'off']
const BACKDROPS: readonly BackdropChoice[] = ['grid', 'city', 'image', 'none']

/** A stored value outside the picker is the default. */
export function parseLook(v: unknown): LookChoice {
  return LOOKS.find(l => l === v) ?? 'grid'
}

export function parseBackdrop(v: unknown): BackdropChoice {
  return BACKDROPS.find(b => b === v) ?? 'grid'
}

const made = new Map<LookVariant, { theme: ThemeColors; problems: string[] }>()

export function colorsFor(variant: LookVariant): { theme: ThemeColors; problems: string[] } {
  let c = made.get(variant)
  if (c === undefined) {
    const s = SURFACES[variant]
    c = makeColors({
      label: LOOK_NAMES[variant],
      only: 'dark',
      roles: { accent: PALETTE.magenta, danger: PALETTE.red, success: PALETTE.mint, warning: PALETTE.yellow, pro: PALETTE.violet },
      dark: {
        gray: {
          anchors: {
            'surface-0': s[0],
            'surface-1': s[1],
            'surface-2': s[2],
            'surface-3': s[3],
            'text-primary': PALETTE.ice,
            'text-secondary': PALETTE.cyanLight,
            'text-muted': PALETTE.dimLight,
          },
        },
        git: {
          added: PALETTE.mint,
          removed: PALETTE.red,
          modified: PALETTE.yellow,
          merged: PALETTE.violet,
          closed: PALETTE.magenta,
          conflicting: PALETTE.gold,
          draft: PALETTE.dimLight,
        },
      },
    })
    made.set(variant, c)
  }
  return c
}

/** `image` is the file used when `backdrop` is `image`. */
export function lookFor(variant: LookVariant, backdrop: BackdropChoice, image: string = IMAGE_FILES[0]): ThemeSetArgs {
  const colors = colorsFor(variant).theme
  switch (backdrop) {
    case 'none':
      return { colors }
    case 'grid':
      return { colors, backdrop: { scene: 'purr-grid', glass: false } }
    case 'city':
      return { colors, backdrop: { scene: 'purr-city', glass: false } }
    case 'image':
      return { colors, backdrop: { image: { source: image }, glass: { blur: 18, tint: SURFACES[variant][0], opacity: 0.72 } } }
  }
}
