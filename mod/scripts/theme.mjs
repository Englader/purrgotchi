// Generates mod/themes/neon-grid.json from the palette in hooks/neon.ts; --check verifies it.
import { execFileSync } from 'node:child_process'
import { existsSync, readFileSync, writeFileSync, mkdirSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import { PALETTE, THEME, THEME_BASE, THEME_NAME } from '../hooks/neon.ts'

const OUT = resolve(dirname(fileURLToPath(import.meta.url)), '../themes/neon-grid.json')
const MAX_BYTES = 262_144

const overrides = Object.fromEntries(Object.entries(THEME).map(([k, p]) => [k, PALETTE[p]]))
const generated = JSON.stringify({ name: THEME_NAME, base: THEME_BASE, overrides }, null, 2) + '\n'

function engineKeys() {
  const bin = process.env.CLAUDE_BIN || execFileSync('sh', ['-c', 'readlink -f "$(which claude)"'], { encoding: 'utf8' }).trim()
  const text = readFileSync(bin).toString('latin1')
  const at = text.indexOf('userMessageBackground:"rgb(55, 55, 55)"')
  if (at < 0) throw new Error('dark palette marker not found in binary')
  const start = text.lastIndexOf('{autoAccept:', at)
  const end = text.indexOf('}', at)
  if (start < 0 || end < 0) throw new Error('dark palette literal not found in binary')
  return [...text.slice(start + 1, end).matchAll(/(?:^|,)(\w+):/g)].map((m) => m[1])
}

function check() {
  const fail = []
  const have = existsSync(OUT) ? readFileSync(OUT, 'utf8') : null
  if (have !== generated) fail.push('theme file differs from generated text (run without --check)')
  const engine = new Set(engineKeys())
  const ours = new Set(Object.keys(THEME))
  const missing = [...engine].filter((k) => !ours.has(k))
  const extra = [...ours].filter((k) => !engine.has(k))
  if (missing.length || extra.length) fail.push(`key mismatch with binary: missing [${missing}] extra [${extra}]`)
  const bad = Object.entries(overrides).filter(([, v]) => !/^#[0-9a-fA-F]{6}$/.test(v))
  if (bad.length) fail.push(`bad hex values: ${bad.map(([k]) => k)}`)
  if (Buffer.byteLength(generated) > MAX_BYTES || (have !== null && Buffer.byteLength(have) > MAX_BYTES)) fail.push('theme file exceeds 262144 bytes')
  if (fail.length) {
    for (const f of fail) console.error(`theme FAIL: ${f}`)
    process.exit(1)
  }
  console.log(`theme ok: ${Object.keys(THEME).length} keys`)
}

if (process.argv.includes('--check')) check()
else {
  mkdirSync(dirname(OUT), { recursive: true })
  writeFileSync(OUT, generated)
}
