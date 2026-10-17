export const W = 32, H = 32

const hex = c => [1, 3, 5].map(i => parseInt(c.slice(i, i + 2), 16))
const toHex = a => '#' + a.map(v => Math.max(0, Math.min(255, Math.round(v))).toString(16).padStart(2, '0')).join('')
export const mix = (a, b, t) => { const x = hex(a), y = hex(b); return toHex(x.map((v, i) => v + (y[i] - v) * t)) }
const lum = c => { const [r, g, b] = hex(c); return (0.299 * r + 0.587 * g + 0.114 * b) / 255 }
const shade = (c, t) => mix(c, '#0d0618', t)
const light = (c, t) => mix(c, '#ffffff', t)

export const NEON = { cyan: '#0abdc6', magenta: '#ea00d9', yellow: '#f3f31b', purple: '#711c91', mint: '#57eca6', red: '#ff5468' }
export const OUTLINE = '#160b26'
const LID = '#160b26'

export const RARITY = {
  common: '#0abdc6', uncommon: '#57eca6', rare: '#b45cff', epic: '#ffc93c', legendary: null,
}
const RAINBOW = ['#ff5468', '#ffc93c', '#f3f31b', '#57eca6', '#0abdc6', '#b45cff', '#ea00d9']

export const BREEDS = {
  'Tabby':            { base: '#9a7b5f', mark: '#5c4332', light: '#e8dccb', pattern: 'tabby' },
  'Tuxedo':           { base: '#24212d', mark: '#24212d', light: '#eeeaf2', pattern: 'tuxedo' },
  'Calico':           { base: '#f1ece4', mark: '#e5893a', mark2: '#2a2530', pattern: 'calico' },
  'Ginger':           { base: '#e8913a', mark: '#c26a22', light: '#f7d9b5', pattern: 'tabby' },
  'Grey Shorthair':   { base: '#8d94a5', mark: '#8d94a5', light: '#c9ced9', pattern: 'solid' },
  'Siamese':          { base: '#efe2cc', mark: '#4a3328', pattern: 'points', bigEars: true },
  'Russian Blue':     { base: '#6e8199', mark: '#6e8199', pattern: 'solid', sheen: 0.22 },
  'Bengal':           { base: '#d9a54c', mark: '#5a3a1e', light: '#f3e2bd', pattern: 'spots' },
  'Maine Coon':       { base: '#8a6a4c', mark: '#4f3a29', light: '#eadfcf', pattern: 'tabby', fluff: true, tufts: true, bushy: true },
  'Norwegian Forest': { base: '#9aa1ad', mark: '#646b78', light: '#f0f0f3', pattern: 'tabby', fluff: true, tufts: true, bushy: true },
  'Turkish Van':      { base: '#f3efe8', mark: '#b5552a', pattern: 'van' },
  'Sphinx':           { base: '#dcab95', mark: '#b98270', pattern: 'wrinkles', bigEars: true, thinTail: true },
  'Scottish Fold':    { base: '#b9bdc9', mark: '#8c91a0', light: '#e3e5ec', pattern: 'tabby', folded: true, round: true },
  'Void Cat':         { base: '#16121f', mark: '#2c2145', pattern: 'void', glow: true },
  'Snow Leopard Cat': { base: '#e2e5ec', mark: '#7b8292', pattern: 'rosettes', fluff: true, bushy: true },
  'Chimera':          { base: '#221d2b', mark: '#e08a3c', pattern: 'split', hetero: true },
}

export const RARITY_OF = {
  Tabby: 'common', Tuxedo: 'common', Calico: 'common', Ginger: 'common', 'Grey Shorthair': 'common',
  Siamese: 'uncommon', 'Russian Blue': 'uncommon', Bengal: 'uncommon',
  'Maine Coon': 'rare', 'Norwegian Forest': 'rare', 'Turkish Van': 'rare',
  Sphinx: 'epic', 'Scottish Fold': 'epic',
  'Void Cat': 'legendary', 'Snow Leopard Cat': 'legendary', Chimera: 'legendary',
}

const STAGES = {
  kitten: { head: [16, 18.6, 9.2, 7.4], body: [16, 27.8, 5.4, 3.9], ear: [6.8, 8.6, 4.6, 16.4, 13.2, 12.2], eyeY: 18.4, eyeDX: 4, tail: [[20, 30], [25.5, 30.5], [25, 25]], paws: 30.8, eyeH: 4 },
  young:  { head: [16, 14.2, 9.4, 7.2], body: [16, 24.6, 7.2, 6.0], ear: [6.8, 3.8, 4.4, 12.4, 13.4, 8.2], eyeY: 13.6, eyeDX: 4.4, tail: [[22, 29], [29.5, 29], [28, 19.5]], paws: 30.5, eyeH: 3 },
  adult:  { head: [16, 11.8, 9.8, 7.4], body: [16, 22.8, 8.8, 7.8], ear: [6.6, 1.2, 4.2, 10.2, 13.6, 5.6], eyeY: 11.8, eyeDX: 4.6, tail: [[23, 29.5], [31, 29.5], [28.5, 16.5]], paws: 30.6, eyeH: 3 },
}

const inEll = (x, y, cx, cy, rx, ry, p = 2) => Math.abs((x + 0.5 - cx) / rx) ** p + Math.abs((y + 0.5 - cy) / ry) ** p <= 1
function inTri(x, y, a, b, c) {
  const px = x + 0.5, py = y + 0.5
  const s = (p1, p2) => (px - p2[0]) * (p1[1] - p2[1]) - (p1[0] - p2[0]) * (py - p2[1])
  const d1 = s(a, b), d2 = s(b, c), d3 = s(c, a)
  return !((d1 < 0 || d2 < 0 || d3 < 0) && (d1 > 0 || d2 > 0 || d3 > 0))
}
const hash = (x, y, s = 0) => { let h = (x * 374761393 + y * 668265263 + s * 2147483647) | 0; h = (h ^ (h >>> 13)) * 1274126177 | 0; return ((h ^ (h >>> 16)) >>> 0) / 4294967296 }

const spot = (x, y, seed, cell) => {
  const gy = Math.floor(y / cell), gx = Math.floor((x + (gy % 2) * cell / 2) / cell)
  const cx = gx * cell - (gy % 2) * cell / 2 + cell / 2 + (hash(gx, gy, seed) - 0.5) * 1.2
  const cy = gy * cell + cell / 2 + (hash(gx, gy, seed + 1) - 0.5) * 1.2
  return { d: Math.hypot(x + 0.5 - cx, y + 0.5 - cy), on: hash(gx, gy, seed + 2) > 0.25, a: Math.atan2(y + 0.5 - cy, x + 0.5 - cx) }
}

export function render({ breed = 'Ginger', stage = 'adult', mood = 'calm', frame = 0, shiny = false, tick = 0 } = {}) {
  const B = BREEDS[breed], S = STAGES[stage], rarity = RARITY_OF[breed]
  const eye = rarity === 'legendary' ? RAINBOW[tick % RAINBOW.length] : RARITY[rarity]
  const reg = Array.from({ length: H }, () => Array(W).fill(null))
  const set = (x, y, r) => { if (x >= 0 && y >= 0 && x < W && y < H) reg[y][x] = r }
  const ox = mood === 'panic' && frame === 1 ? 1 : 0
  const oy = mood === 'sleep' ? 1 : 0
  let [hx, hy, hrx, hry] = S.head; hy += oy; hx += ox
  const [bx, by, brx, bry] = S.body
  const hp = B.round ? 2.0 : B.fluff ? 2.5 : 2.25
  if (B.fluff) hrx += 0.4

  // tail (behind body)
  const [p0, p1, p2] = S.tail
  const sway = mood === 'happy' ? (frame ? -1.5 : 0) : frame ? 1 : 0
  const end = [p2[0] + sway, p2[1] - (mood === 'happy' ? 2 : 0)]
  const tr = B.bushy ? 2.3 : B.thinTail ? 1.05 : 1.6
  for (let t = 0; t <= 1; t += 0.02) {
    const cx = (1 - t) ** 2 * p0[0] + 2 * (1 - t) * t * p1[0] + t * t * end[0]
    const cy = (1 - t) ** 2 * p0[1] + 2 * (1 - t) * t * p1[1] + t * t * end[1]
    const r = tr * (B.thinTail ? 1 : 0.85 + 0.25 * Math.sin(t * Math.PI))
    for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) if (inEll(x, y, cx, cy, r, r)) set(x, y, 'tail')
  }
  for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) if (inEll(x, y, bx, by, brx, bry, 2.2)) set(x, y, 'body')
  for (const dx of [-3.6, 3.6]) for (let y = 0; y < H; y++) for (let x = 0; x < W; x++)
    if (inEll(x, y, bx + dx, S.paws, 2.6, 1.5)) set(x, y, 'paw')

  // ears
  let [tx, ty, b1x, b1y, b2x, b2y] = S.ear
  ty += oy; b1y += oy; b2y += oy; tx += ox; b1x += ox; b2x += ox
  b1x = Math.max(b1x, hx - hrx + 0.9)
  if (B.bigEars) { tx -= 0.8; ty -= 1.4; b1x -= 0.3 }
  if (mood === 'alert' && frame === 1) tx -= 1
  if (mood === 'sleep' || mood === 'hungry') ty += 1
  if (B.folded) { ty = b2y + 1.2; tx = b1x + 3.4 }
  const earL = [[tx, ty], [b1x, b1y], [b2x, b2y]]
  const earR = earL.map(([x, y]) => [32 - x + 2 * ox, y])
  for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
    if (inTri(x, y, ...earL) || inTri(x, y, ...earR)) set(x, y, 'ear')
  }
  if (B.tufts && !B.folded) { set(Math.round(tx), Math.floor(ty) - 1, 'ear'); set(Math.round(32 - tx + 2 * ox) - 1, Math.floor(ty) - 1, 'ear') }
  // head
  for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) if (inEll(x, y, hx, hy, hrx, hry, hp)) set(x, y, 'head')
  if (B.fluff) for (const side of [-1, 1]) for (let k = 0; k < 3; k++) {
    const x = Math.round(hx + side * (hrx + 0.2)) - (side > 0 ? 1 : 0), y = Math.round(hy + 2 + k * 1.5)
    if (k !== 1) set(x, y, 'head')
  }
  // inner ear
  const shrink = (tri, f) => { const c = [(tri[0][0] + tri[1][0] + tri[2][0]) / 3, (tri[0][1] + tri[1][1] + tri[2][1]) / 3 + 0.6]; return tri.map(([x, y]) => [c[0] + (x - c[0]) * f, c[1] + (y - c[1]) * f]) }
  if (!B.folded) for (const tri of [shrink(earL, 0.62), shrink(earR, 0.62)])
    for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) if (reg[y][x] === 'ear' && inTri(x, y, ...tri)) set(x, y, 'inner')
  // chest + muzzle
  const ny = Math.round(hy + 3.4 - (stage === 'kitten' ? 0.4 : 0))
  for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
    if (reg[y][x] === 'body' && inEll(x, y, bx, by - 1, brx * (B.fluff ? 0.55 : 0.42), bry * 0.8)) reg[y][x] = 'chest'
    if (reg[y][x] === 'head' && inEll(x, y, hx, ny + 1.2, 2.7, 1.7)) reg[y][x] = 'muzzle'
  }

  // coat colour
  const col = Array.from({ length: H }, () => Array(W).fill(null))
  const lightC = B.light ?? B.base
  for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
    const r = reg[y][x]; if (!r) continue
    let c = B.base
    if (r === 'inner') c = '#f39bb8'
    else if ((r === 'chest' || r === 'paw') && B.light) c = lightC
    else if (r === 'muzzle' && B.light) c = mix(B.base, lightC, 0.7)
    const lx = x - ox
    switch (B.pattern) {
      case 'tabby': {
        if (r === 'head' && y < hy - 2.5 && Math.abs(lx - 16) < 5 && (lx - 16) % 2 === 0) c = B.mark
        if ((r === 'head') && Math.abs(lx - 16) > hrx - 3.5 && y > hy - 1 && y % 2 === 0) c = B.mark
        if (r === 'body' && Math.floor(y + Math.abs(lx - 16) * 0.55) % 3 === 0 && Math.abs(lx - 16) > 2) c = B.mark
        if (r === 'tail' && (x + y) % 3 === 0) c = B.mark
        break
      }
      case 'tuxedo':
        if (r === 'head' && Math.abs(lx - 16) < 1.5 && y > hy - 2) c = B.light
        if (r === 'tail' && y > 28) c = B.light
        break
      case 'calico': {
        const blobs = [[9, hy - 4, 4.2, 'mark'], [22, hy - 3, 3.6, 'mark2'], [10, by + 1, 3.6, 'mark2'], [23, by + 2, 4, 'mark'], [28, 20, 3, 'mark']]
        for (const [cx, cy, rr, k] of blobs) if (r !== 'inner' && inEll(lx, y, cx, cy, rr, rr * 0.85)) c = B[k]
        break
      }
      case 'points': {
        if (r === 'ear' || r === 'tail' || r === 'paw') c = B.mark
        else if ((r === 'muzzle' || r === 'head') && inEll(lx, y, 16, ny - 0.8, 4.4, 3.6)) c = mix(B.base, B.mark, r === 'muzzle' ? 0.9 : 0.6)
        else if (r === 'tail') c = B.mark
        break
      }
      case 'spots':
        { const sp = spot(x, y, 3, 4); if ((r === 'body' || r === 'tail' || (r === 'head' && y < hy - 1)) && sp.on && sp.d < 1.25) c = sp.d < 0.6 ? mix(B.mark, B.base, 0.45) : B.mark }
        if (r === 'head' && y < hy - 2.5 && Math.abs(lx - 16) < 4 && (lx - 16) % 2 === 0) c = B.mark
        break
      case 'van':
        if ((r === 'ear' || r === 'head') && y < hy - 2 && Math.abs(lx - 16) > 1.6) c = B.mark
        if (r === 'tail') c = (x + y) % 4 === 0 ? mix(B.mark, '#000000', 0.25) : B.mark
        break
      case 'wrinkles':
        if (r === 'head' && y < hy - 2.5 && Math.abs(lx - 16) < 3.5 && y % 2 === 1) c = B.mark
        if (r === 'body' && Math.abs(lx - 16) > brx - 3 && y % 3 === 0) c = B.mark
        break
      case 'void':
        if (r !== 'inner' && hash(x, y, 9) > 0.965) c = hash(x, y, 4) > 0.5 ? NEON.cyan : NEON.magenta
        if (r === 'inner') c = '#3a2a5e'
        break
      case 'rosettes': {
        const sp = spot(x, y, 7, 5)
        if (r !== 'inner' && r !== 'muzzle' && r !== 'chest' && sp.on && sp.d > 0.9 && sp.d < 1.9 && Math.sin(sp.a * 3 + 1) > -0.4) c = B.mark
        if (r === 'head' && y < hy - 2 && sp.on && sp.d < 0.8) c = B.mark
        if (r === 'tail' && (x + y) % 4 < 2) c = B.mark
        break
      }
      case 'split':
        if (r !== 'inner') c = lx < 16 ? B.base : B.mark
        if (r === 'muzzle') c = lx < 16 ? '#3a3346' : '#f3c99a'
        break
    }
    col[y][x] = c
  }

  // shading: highlight up-left, shadow down-right, chin occlusion
  const filled = (x, y) => x >= 0 && y >= 0 && x < W && y < H && reg[y][x]
  const out = Array.from({ length: H }, () => Array(W).fill(''))
  const sheen = (B.sheen ?? 0.12) + (shiny ? 0.12 : 0)
  for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
    const r = reg[y][x]; if (!r) continue
    let c = col[y][x]
    if (!filled(x - 1, y) || !filled(x, y - 1)) c = light(c, sheen)
    if (!filled(x + 1, y) || !filled(x, y + 1)) c = shade(c, 0.28)
    else if (!filled(x + 2, y) || !filled(x, y + 2)) c = shade(c, 0.12)
    if ((r === 'body' || r === 'chest') && ['head', 'muzzle'].includes(reg[y - 1]?.[x])) c = shade(c, 0.35)
    if (r === 'tail' && ['body', 'chest', 'paw'].includes(reg[y]?.[x - 1])) c = shade(c, 0.3)
    if (r === 'paw' && reg[y][x - 1] === 'paw' && reg[y][x + 1] !== 'paw') c = shade(c, 0.2)
    out[y][x] = c
  }
  // outline + neon rim (cyan left, magenta right)
  for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
    if (reg[y][x]) {
      if (!filled(x - 1, y) && x < 16 + ox) out[y][x] = mix(out[y][x], NEON.cyan, 0.45)
      if (!filled(x + 1, y) && x >= 16 + ox) out[y][x] = mix(out[y][x], NEON.magenta, 0.45)
      continue
    }
    if (filled(x - 1, y) || filled(x + 1, y) || filled(x, y - 1) || filled(x, y + 1)) out[y][x] = OUTLINE
  }
  // paw separators
  for (const dx of [-3.6, 3.6]) { const x = Math.round(bx + dx), y = Math.round(S.paws) ; if (reg[y]?.[x] === 'paw') out[y][x] = shade(out[y][x], 0.45) }

  // collar in rarity colour
  const cy = Math.round(hy + hry) - 1
  for (let x = 0; x < W; x++) if (['body', 'chest'].includes(reg[cy + 1]?.[x]) && ['head', 'muzzle'].includes(reg[cy]?.[x]) && Math.abs(x - 16 - ox) < 5) out[cy + 1][x] = mix(eye, '#000000', 0.15)
  out[cy + 2] && (out[cy + 2][16 + ox] = eye)

  // face
  const ey = Math.round(hy + (S.eyeY - S.head[1]) - 1) - (stage === 'kitten' ? 0 : 0)
  const eyes = [...{
    alert:  ['wEE', 'EkE', 'EEE'],
    hungry: ['wEE', 'EkE', 'EkE'],
    calm:   frame ? ['...', '...', 'LLL'] : ['...', 'LLL', 'EkE'],
    happy:  ['...', '.L.', 'L.L'],
    panic:  ['WWW', 'WkW', 'WWW'],
    sleep:  ['...', '...', 'LLL'],
  }[mood]]
  if (S.eyeH === 4 && eyes.join('').includes('E') && eyes[1].includes('k')) eyes.splice(1, 0, eyes[1].replace('k', 'E'))
  const eyeCol = (side) => B.hetero && side > 0 ? NEON.magenta : eye
  for (const side of [-1, 1]) {
    const ex = Math.round(hx + side * S.eyeDX) - 1
    eyes.forEach((row, j) => [...row].forEach((ch, i) => {
      const x = ex + i, y = ey + j
      if (ch === '.') return
      const under = out[y]?.[x] || B.base
      const c = { E: eyeCol(side), k: '#0b0612', w: '#ffffff', W: '#f4f1ff', L: lum(under) < 0.3 ? mix(eyeCol(side), '#ffffff', 0.35) : LID }[ch]
      if (out[y]) out[y][x] = ch === 'E' && B.glow ? light(c, 0.15) : c
    }))
    if (B.glow && ['alert', 'hungry', 'calm', 'panic'].includes(mood)) {
      const gx = ex + (side < 0 ? -1 : 3); if (out[ey + 1] && reg[ey + 1][gx]) out[ey + 1][gx] = mix(out[ey + 1][gx], eyeCol(side), 0.5)
    }
  }
  const nx = Math.round(hx) - 1
  const nose = '#ff7aa8'
  out[ny][nx] = nose; out[ny][nx + 1] = nose
  const mouth = (pts, c = LID) => pts.forEach(([dx, dy]) => { const row = out[ny + dy]; if (!row) return; const u = row[nx + dx] || B.base; row[nx + dx] = c === LID && lum(u) < 0.3 ? '#c9c3dd' : c })
  if (mood === 'happy') { mouth([[-1, 1], [2, 1], [0, 1], [1, 1]]); mouth([[0, 2], [1, 2]], '#ff7aa8') }
  else if (mood === 'panic') mouth([[0, 1], [1, 1], [0, 2], [1, 2]], '#2a0f2a')
  else if (mood === 'hungry') { mouth([[0, 1], [1, 1]], '#2a0f2a'); mouth([[1, 2]], '#ff7aa8'); if (frame) mouth([[0, 3]], NEON.cyan) }
  else mouth([[-1, 1], [2, 1], [0, 1], [1, 1]].filter(([dx]) => dx === -1 || dx === 2))
  // mood props
  const glyph = (rows, x0, y0, c) => rows.forEach((row, j) => [...row].forEach((ch, i) => { if (ch === '#' && out[y0 + j] && x0 + i < W && x0 + i >= 0) out[y0 + j][x0 + i] = c }))
  if (mood === 'sleep') { glyph(['####', '..#.', '.#..', '####'], 27, frame ? 0 : 1, NEON.cyan); glyph(['##.', '.#.', '.##'], frame ? 29 : 28, frame ? 6 : 7, mix(NEON.cyan, '#000000', 0.35)) }
  if (mood === 'happy' && frame) glyph(['#.#', '###', '.#.'], 26, 3, NEON.magenta)
  if (mood === 'panic') glyph(['.#.', '###', '###', '.#.'], 28, 6 + frame, NEON.cyan)

  // rarity aura / shiny sparkle
  const sparks = { rare: 2, epic: 3, legendary: 4 }[rarity] ?? 0
  const spots = [[2, 4], [28, 11], [3, 18], [29, 25], [1, 27]]
  for (let i = 0; i < sparks + (shiny ? 2 : 0); i++) {
    const [x, y] = spots[(i + frame) % spots.length]
    if (reg[y]?.[x] || out[y][x]) continue
    const c = shiny && i >= sparks ? '#ffffff' : rarity === 'legendary' ? RAINBOW[(tick + i) % RAINBOW.length] : eye
    out[y][x] = c
    if ((i + frame) % 2 === 0) for (const [dx, dy] of [[1, 0], [-1, 0], [0, 1], [0, -1]]) if (out[y + dy] && !out[y + dy][x + dx] && !reg[y + dy]?.[x + dx]) out[y + dy][x + dx] = mix(c, '#000000', 0.45)
  }
  return { w: W, h: H, px: out }
}
