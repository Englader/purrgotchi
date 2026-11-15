import { expect, test } from 'claude-code/testing'

import { SURFACES } from './look'
import { SCENE_ALT, isScene, sceneSvg } from './scenes'

const SCENES = ['purr-grid', 'purr-city'] as const
const VARIANTS = ['grid', 'violet'] as const

test('every scene is a self-contained svg within the limit', async () => {
  for (const s of SCENES) {
    for (const v of VARIANTS) {
      const svg = sceneSvg(s, v)
      expect(svg.startsWith('<svg')).toBe(true)
      expect(svg.length).toBeLessThan(131_072)
      expect(svg).toContain('viewBox="0 0 1200 800"')
      expect(svg).toContain('preserveAspectRatio="xMidYMax slice"')
      expect(svg).toContain('prefers-reduced-motion')
      expect(svg).toContain('@media (max-aspect-ratio:4/3){.cat-wide{display:none}')
      expect(svg).toContain(SURFACES[v][0])
      expect(/<script/i.test(svg)).toBe(false)
      expect(/https?:/.test(svg.replace('xmlns="http://www.w3.org/2000/svg"', ''))).toBe(false)
      expect(/url\(\s*['"]?(?!#)/.test(svg)).toBe(false)
      expect(/NaN|undefined/.test(svg)).toBe(false)
    }
    expect(SCENE_ALT[s].length).toBeGreaterThan(10)
  }
})

test('the variants and the scenes differ, and a scene is stable', async () => {
  expect(sceneSvg('purr-grid', 'grid')).not.toBe(sceneSvg('purr-grid', 'violet'))
  expect(sceneSvg('purr-grid', 'grid')).not.toBe(sceneSvg('purr-city', 'grid'))
  expect(sceneSvg('purr-city', 'violet')).toBe(sceneSvg('purr-city', 'violet'))
})

test('the floor gradient is in user space, so the vertical ray draws', async () => {
  expect(sceneSvg('purr-grid', 'grid')).toContain('<linearGradient id="pg-floor" gradientUnits="userSpaceOnUse"')
})

test('isScene knows only our scenes', async () => {
  expect(isScene('purr-grid')).toBe(true)
  expect(isScene('purr-city')).toBe(true)
  expect(isScene('lagoon')).toBe(false)
  expect(isScene('')).toBe(false)
})
