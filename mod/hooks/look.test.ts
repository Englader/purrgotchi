import { expect, test } from 'claude-code/testing'

import { IMAGE_FILES, SURFACES, colorsFor, lookFor, parseBackdrop, parseLook } from './look'

const VARIANTS = ['grid', 'violet'] as const

test('each variant paints', async () => {
  for (const v of VARIANTS) {
    const { theme, problems } = colorsFor(v)
    expect(problems.filter(l => l.startsWith("Desktop won't paint"))).toEqual([])
    expect((theme as { inputs: { dark: Record<string, string> } }).inputs.dark['gray-900']).toBe(SURFACES[v][0])
  }
})

test('parse falls back to defaults', async () => {
  expect(parseLook('violet')).toBe('violet')
  expect(parseLook('off')).toBe('off')
  expect(parseLook('nope')).toBe('grid')
  expect(parseLook(undefined)).toBe('grid')
  expect(parseLook(3)).toBe('grid')
  expect(parseBackdrop('city')).toBe('city')
  expect(parseBackdrop('image')).toBe('image')
  expect(parseBackdrop('none')).toBe('none')
  expect(parseBackdrop('nope')).toBe('grid')
  expect(parseBackdrop(undefined)).toBe('grid')
})

test('lookFor shapes', async () => {
  for (const v of VARIANTS) {
    expect(lookFor(v, 'none').backdrop).toBeUndefined()
    expect(lookFor(v, 'grid').backdrop).toEqual({ scene: 'purr-grid', glass: false })
    expect(lookFor(v, 'city').backdrop).toEqual({ scene: 'purr-city', glass: false })
    expect(lookFor(v, 'image', 'backdrops/custom.png').backdrop).toEqual({ image: { source: 'backdrops/custom.png' } })
    expect(lookFor(v, 'image').backdrop).toMatchObject({ image: { source: IMAGE_FILES[0] } })
    expect(lookFor(v, 'grid').colors).toEqual(colorsFor(v).theme)
  }
})
