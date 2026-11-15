// The desktop backdrop scenes, as SVG text: a neon grid horizon and a neon city skyline, each with the cat. Pure: no `$`.
import { SURFACES } from './look'
import type { LookVariant } from './look'
import { PALETTE } from './neon'

export type SceneName = 'purr-grid' | 'purr-city'

export const SCENE_ALT: Readonly<Record<SceneName, string>> = {
  'purr-grid': 'A neon grid running to a low sun on the horizon, with a small pixel cat sitting on the line',
  'purr-city': 'A neon city skyline at night with lit windows, and a small pixel cat sitting on a rooftop',
}

export function isScene(name: string): name is SceneName {
  return Object.hasOwn(SCENE_ALT, name)
}

const W = 1200
const H = 800
/** The horizon: the words sit above it, the prompt box below. */
const HORIZON = 470

const n = (v: number) => String(Math.round(v * 10) / 10)

/** The sitting cat, 14 x 12 pixels: `#` body, `o` eye, `t` tail. */
const CAT = [
  '..#......#....',
  '..##....##....',
  '..########....',
  '..#o####o#....',
  '..########....',
  '...######.....',
  '..########..t.',
  '.##########.t.',
  '.##########.t.',
  '.##########tt.',
  '.#########tt..',
  '..###..###....',
]
const CAT_PX = 5

/** The cat with its feet on (`x`, `y`), `x` its left edge. `where` says which box shape shows it. */
function cat(x: number, y: number, ground: string, where: 'wide' | 'narrow'): string {
  const top = y - CAT.length * CAT_PX
  const cell = (c: number, r: number, extra = '') => `<rect x="${x + c * CAT_PX}" y="${top + r * CAT_PX}" width="${CAT_PX}" height="${CAT_PX}"${extra}/>`
  let body = ''
  let eyes = ''
  let tail = ''
  CAT.forEach((row, r) => {
    ;[...row].forEach((ch, c) => {
      if (ch === '#') body += cell(c, r)
      else if (ch === 'o') eyes += cell(c, r)
      else if (ch === 't') tail += cell(c, r)
    })
  })
  // The tail swings about its root, the bottom-left of the tail cells.
  const ox = x + 10 * CAT_PX
  const oy = top + 11 * CAT_PX
  return (
    `<g class="cat-${where}" fill="${PALETTE.cyan}" shape-rendering="crispEdges">` +
    `<g class="tail" style="transform-origin:${ox}px ${oy}px">${tail}</g>${body}` +
    `<g fill="${PALETTE.cyan}">${eyes}</g><g class="eyes" fill="${ground}">${eyes}</g></g>`
  )
}

const CAT_CSS =
  // A box narrower than 4:3 crops the sides past the right-hand cat: a second one nearer the middle takes over.
  '.cat-narrow{display:none}@media (max-aspect-ratio:4/3){.cat-wide{display:none}.cat-narrow{display:inline}}' +
  '@keyframes purr-tail{0%,70%,100%{transform:rotate(0)}80%{transform:rotate(9deg)}90%{transform:rotate(-5deg)}}' +
  '@keyframes purr-blink{0%,92%,100%{opacity:1}95%{opacity:0}}'
const CAT_MOTION = '.tail{animation:purr-tail 7s ease-in-out infinite}.eyes{animation:purr-blink 6s steps(1) infinite}'

function open(ground: string, css: string, motion: string, defs: string): string {
  return (
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${W} ${H}" preserveAspectRatio="xMidYMax slice">` +
    `<style>${CAT_CSS}${css}@media (prefers-reduced-motion:no-preference){${CAT_MOTION}${motion}}</style>` +
    `<defs>${defs}</defs><rect width="${W}" height="${H}" fill="${ground}"/>`
  )
}

const stop = (offset: number, color: string, opacity: number) => `<stop offset="${offset}" stop-color="${color}" stop-opacity="${opacity}"/>`
const vertical = (id: string, stops: string) => `<linearGradient id="${id}" x1="0" y1="0" x2="0" y2="1">${stops}</linearGradient>`

function grid(ground: string): string {
  const defs =
    vertical('pg-sky', stop(0.3, PALETTE.purple, 0) + stop(1, PALETTE.purple, 0.4)) +
    vertical('pg-sun', stop(0, PALETTE.yellow, 1) + stop(1, PALETTE.magenta, 1)) +
    // In user space: a vertical ray has no box of its own for a gradient to fill.
    `<linearGradient id="pg-floor" gradientUnits="userSpaceOnUse" x1="0" y1="${HORIZON}" x2="0" y2="${H}">${stop(0, PALETTE.magenta, 0.05) + stop(1, PALETTE.magenta, 0.42)}</linearGradient>` +
    `<clipPath id="pg-above"><rect width="${W}" height="${HORIZON}"/></clipPath>`
  // The sun's lower half is cut by widening bands of ground.
  let bands = ''
  for (let i = 0; i < 5; i++) bands += `<rect x="780" y="${HORIZON - 14 - i * 16}" width="240" height="${6 - i}" fill="${ground}"/>`
  let rays = ''
  for (let i = -16; i <= 16; i++) rays += `<line x1="${W / 2 + i * 16}" y1="${HORIZON}" x2="${W / 2 + i * 170}" y2="${H}"/>`
  // Each cross line eases down to the next one's place, so the loop is seamless.
  const ROWS = 9
  const y = (j: number) => HORIZON + (H - HORIZON) * Math.pow(j / ROWS, 1.9)
  let rows = ''
  for (let j = 0; j < ROWS; j++) {
    rows += `<line class="row${j === 0 ? ' first' : ''}" style="--d:${n(y(j + 1) - y(j))}px" x1="0" y1="${n(y(j))}" x2="${W}" y2="${n(y(j))}"/>`
  }
  return (
    open(
      ground,
      '',
      '@keyframes purr-row{to{transform:translateY(var(--d))}}@keyframes purr-in{from{opacity:0}}' +
        '.row{animation:purr-row 5s linear infinite}.first{animation:purr-row 5s linear infinite,purr-in 5s linear infinite}',
      defs,
    ) +
    `<rect width="${W}" height="${HORIZON}" fill="url(#pg-sky)"/>` +
    `<g clip-path="url(#pg-above)" opacity="0.5"><circle cx="900" cy="${HORIZON}" r="96" fill="url(#pg-sun)"/>${bands}</g>` +
    `<g stroke="url(#pg-floor)" stroke-width="1.2">${rays}</g>` +
    `<g stroke="${PALETTE.magenta}" stroke-opacity="0.26" stroke-width="1">${rows}</g>` +
    `<line x1="0" y1="${HORIZON}" x2="${W}" y2="${HORIZON}" stroke="${PALETTE.cyan}" stroke-opacity="0.55" stroke-width="1.5"/>` +
    cat(1040, HORIZON, ground, 'wide') +
    cat(690, HORIZON, ground, 'narrow') +
    '</svg>'
  )
}

/** mulberry32: the skyline is the same on every draw. */
function rng(seed: number): () => number {
  let a = seed
  return () => {
    a = (a + 0x6d2b79f5) | 0
    let t = Math.imul(a ^ (a >>> 15), 1 | a)
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}

function city(ground: string, near: string): string {
  const rand = rng(0x70757272)
  const LIGHTS = [PALETTE.cyan, PALETTE.yellow, PALETTE.magenta, PALETTE.violet]
  const defs =
    vertical('pc-sky', stop(0.2, PALETTE.purple, 0) + stop(1, PALETTE.purple, 0.34)) +
    vertical('pc-haze', stop(0, PALETTE.magenta, 0) + stop(1, PALETTE.magenta, 0.2))
  // Far to near: lower tops, wider blocks, more solid.
  const LAYERS = [
    { top: [HORIZON, 600], width: [34, 70], fill: PALETTE.purple, opacity: 0.2, lit: 0.1 },
    { top: [560, 680], width: [50, 100], fill: PALETTE.purple, opacity: 0.36, lit: 0.16 },
    { top: [650, 740], width: [70, 140], fill: near, opacity: 1, lit: 0.22 },
  ] as const
  let out = ''
  // The cats' rooftops: the nearest layer's blocks under x = 1075 and x = 640.
  const roofs = { wide: { at: 1075, x: 1040, y: 700 }, narrow: { at: 640, x: 610, y: 700 } }
  LAYERS.forEach((layer, depth) => {
    let blocks = ''
    let lights = ''
    for (let x = -20; x < W; ) {
      const w = layer.width[0] + rand() * (layer.width[1] - layer.width[0])
      const top = layer.top[0] + rand() * (layer.top[1] - layer.top[0])
      blocks += `<rect x="${n(x)}" y="${n(top)}" width="${n(w + 1)}" height="${n(H - top)}"/>`
      if (depth === 2) {
        for (const r of Object.values(roofs)) {
          if (x <= r.at && x + w > r.at) {
            r.x = Math.min(Math.max(x, r.at - 35), x + w - 70)
            r.y = top
          }
        }
      }
      for (let wy = top + 10; wy < H - 12; wy += 14) {
        for (let wx = x + 7; wx < x + w - 9; wx += 11) {
          if (rand() < layer.lit) {
            const color = LIGHTS[Math.floor(rand() * LIGHTS.length)] ?? PALETTE.cyan
            lights += `<rect${rand() < 0.06 ? ' class="fl"' : ''} x="${n(wx)}" y="${n(wy)}" width="5" height="7" fill="${color}"/>`
          }
        }
      }
      x += w
    }
    out += `<g fill="${layer.fill}" opacity="${layer.opacity}">${blocks}</g><g opacity="${0.3 + depth * 0.12}">${lights}</g>`
    if (depth === 0) out += `<rect y="${HORIZON}" width="${W}" height="${H - HORIZON}" fill="url(#pc-haze)"/>`
  })
  return (
    open(ground, '', '@keyframes purr-flicker{0%,100%{opacity:1}45%{opacity:1}50%{opacity:.15}55%{opacity:1}}.fl{animation:purr-flicker 9s steps(1) infinite}', defs) +
    `<rect width="${W}" height="${H}" fill="url(#pc-sky)"/>` +
    out +
    cat(Math.round(roofs.wide.x), Math.round(roofs.wide.y), near, 'wide') +
    cat(Math.round(roofs.narrow.x), Math.round(roofs.narrow.y), near, 'narrow') +
    '</svg>'
  )
}

const drawn = new Map<string, string>()

export function sceneSvg(name: SceneName, variant: LookVariant): string {
  const key = `${name}:${variant}`
  let svg = drawn.get(key)
  if (svg === undefined) {
    const s = SURFACES[variant]
    svg = name === 'purr-grid' ? grid(s[0]) : city(s[0], s[1])
    drawn.set(key, svg)
  }
  return svg
}
