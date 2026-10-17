import { expect, test } from 'claude-code/testing'

import { base64, cropRows, hexToU32, packRaster, svgSource } from './pixels'
import { pixelFixtures as fixtures } from './fixtures/pixels'
import type { PixelGrid } from './pixels'

// The pixel packers are pure, so they are unit-tested directly; the fixtures
// are real grids recorded from the binary (mod/scripts/record-pixel-fixtures.sh).

const DEFAULT = 0x01000000

const bytes = (s: string): Uint8Array => Uint8Array.from([...s].map(c => c.charCodeAt(0)))

/** An independent RFC 4648 decoder (so `base64` is not checked against itself). */
function decode(b64: string): Uint8Array {
  const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/'
  const out: number[] = []
  let acc = 0
  let bits = 0
  for (const ch of b64.replace(/=+$/, '')) {
    acc = (acc << 6) | alphabet.indexOf(ch)
    bits += 6
    if (bits >= 8) {
      bits -= 8
      out.push((acc >> bits) & 0xff)
    }
  }
  return Uint8Array.from(out)
}

function u32s(b64: string): number[] {
  const b = decode(b64)
  const v = new DataView(b.buffer, b.byteOffset, b.byteLength)
  const out: number[] = []
  for (let i = 0; i + 4 <= b.length; i += 4) out.push(v.getUint32(i, true))
  return out
}

test('base64 vectors', () => {
  expect(base64(new Uint8Array(0))).toBe('')
  expect(base64(bytes('M'))).toBe('TQ==')
  expect(base64(bytes('Ma'))).toBe('TWE=')
  expect(base64(bytes('Man'))).toBe('TWFu')
  expect(base64(bytes('Many hands'))).toBe('TWFueSBoYW5kcw==')
})

test('base64 round-trips 300 pseudo-random bytes', () => {
  let s = 12345
  const b = Uint8Array.from({ length: 300 }, () => {
    s = (s * 1103515245 + 12345) & 0x7fffffff
    return s & 0xff
  })
  expect([...decode(base64(b))]).toEqual([...b])
})

test('hexToU32', () => {
  expect(hexToU32('#ea00d9')).toBe(0x00ea00d9)
  expect(hexToU32('#000000')).toBe(0)
  expect(hexToU32(null)).toBe(DEFAULT)
})

test('packRaster cell table: half-blocks, transparent halves, blank', () => {
  const both: PixelGrid = [
    ['#ff0000', '#ff0000', null, null],
    ['#0000ff', null, '#00ff00', null],
  ]
  const r = packRaster(both)
  expect(r.columns).toBe(4)
  expect(r.rows).toBe(1)
  expect(u32s(r.cells)).toEqual([
    // top and bottom set: ▀ fg = top, bg = bottom
    0x2580, 0xff0000, 0x0000ff,
    // top set only: ▀ fg = top, bg = default
    0x2580, 0xff0000, DEFAULT,
    // bottom set only: ▄ fg = bottom, never the default foreground
    0x2584, 0x00ff00, DEFAULT,
    // both transparent: a space
    0x20, DEFAULT, DEFAULT,
  ])
})

test('packRaster on a 2x2 grid and an all-transparent grid', () => {
  const g: PixelGrid = [
    ['#ff0000', null],
    [null, '#00ff00'],
  ]
  const r = packRaster(g, 0, 2)
  expect(r.columns).toBe(2)
  expect(u32s(r.cells)).toEqual([0x2580, 0xff0000, DEFAULT, 0x2584, 0x00ff00, DEFAULT])
  const nothing: PixelGrid = [
    [null, null],
    [null, null],
  ]
  expect(u32s(packRaster(nothing).cells)).toEqual([0x20, DEFAULT, DEFAULT, 0x20, DEFAULT, DEFAULT])
})

test('packRaster windows rows and treats a row past the end as transparent', () => {
  const g: PixelGrid = [['#111111'], ['#222222'], ['#333333']]
  const r = packRaster(g, 1, 3)
  expect(r.rows).toBe(1)
  expect(u32s(r.cells)).toEqual([0x2580, 0x222222, 0x333333])
  const odd = packRaster(g, 2, 3)
  expect(odd.rows).toBe(1)
  expect(u32s(odd.cells)).toEqual([0x2580, 0x333333, DEFAULT])
})

test('cropRows: first painted row, window of 20, never past 32', () => {
  const blank = (): PixelGrid => Array.from({ length: 32 }, () => Array.from({ length: 32 }, () => null as string | null))
  expect(cropRows(blank())).toEqual([0, 20])
  const a = blank()
  a[5]![3] = '#ffffff'
  expect(cropRows(a)).toEqual([5, 25])
  const b = blank()
  b[28]![0] = '#ffffff'
  expect(cropRows(b)).toEqual([12, 32])
  expect(cropRows(a, 8)).toEqual([5, 13])
})

test('svgSource merges a run of equal pixels into one rect', () => {
  const row: PixelGrid = [Array.from({ length: 32 }, () => '#0abdc6')]
  const s = svgSource(row)
  expect(s.match(/<rect/g)).toHaveLength(1)
  expect(s).toContain('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 1" shape-rendering="crispEdges">')
  expect(s).toContain('<rect x="0" y="0" width="32" height="1" fill="#0abdc6"/>')
  expect(s.endsWith('</svg>')).toBe(true)
})

test('svgSource skips null, splits runs and honours the row window', () => {
  const g: PixelGrid = [
    ['#111111', null, '#111111', '#222222'],
    ['#333333', '#333333', null, null],
  ]
  const s = svgSource(g, 1, 2)
  expect(s).toContain('viewBox="0 1 4 1"')
  expect(s.match(/<rect/g)).toHaveLength(1)
  expect(s).toContain('<rect x="0" y="1" width="2" height="1" fill="#333333"/>')
  const all = svgSource(g)
  expect(all.match(/<rect/g)).toHaveLength(4)
  expect(all).not.toContain('null')
})

test('fixtures: six 32x32 grids recorded from the binary', () => {
  expect(fixtures).toHaveLength(6)
  for (const f of fixtures) {
    expect(f.pixels).toHaveLength(32)
    for (const row of f.pixels) expect(row).toHaveLength(32)
  }
})

for (const f of fixtures) {
  test(`fixture "${f.label}" packs within every surface limit`, () => {
    const r = packRaster(f.pixels)
    expect(r.columns).toBe(32)
    expect(r.rows).toBe(16)
    const bytesLen = decode(r.cells).length
    expect(bytesLen).toBe(32 * 16 * 12)
    const words = u32s(r.cells)
    const pairs = new Set<string>()
    for (let i = 0; i < words.length; i += 3) pairs.add(`${words[i + 1]}/${words[i + 2]}`)
    expect(pairs.size).toBeLessThanOrEqual(1024)
    // every code point is the half-block set (one printable width-1 character)
    for (let i = 0; i < words.length; i += 3) expect([0x20, 0x2580, 0x2584]).toContain(words[i])

    expect(svgSource(f.pixels).length).toBeLessThan(131072)

    const [r0, r1] = cropRows(f.pixels)
    expect(r1 - r0).toBe(20)
    expect(r0).toBeGreaterThanOrEqual(0)
    expect(r1).toBeLessThanOrEqual(32)
    const portrait = packRaster(f.pixels, r0, r1)
    expect(portrait.rows).toBe(10)
    expect(svgSource(f.pixels, r0, r1)).toContain(`viewBox="0 ${r0} 32 20"`)
  })
}
