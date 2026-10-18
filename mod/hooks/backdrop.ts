// Neon Grid backdrop: the terminal's background goes to the active Neon theme's ground (OSC 11) while one is on, back to its own (OSC 111) after.
import type { EngineInterface, On } from 'claude-code'

import { locateBinary } from './locate'
import { GROUNDS } from './neon'

/** `null` is the reset. */
type Ground = string | null

const paintSeq = (ground: string) => `\\033]11;${ground}\\007`
const RESET = '\\033]111\\007'
/** session.end shares ~1.5 s with every other hook: one short call. */
const SEND_MS = 1000

// The mod has no raw terminal write and a child has no tty: it walks up its parents to the first with one (Claude Code) and writes there.
const SEND =
  'p=$$; while [ "$p" -gt 1 ]; do t=$(ps -o tty= -p "$p" | tr -d \' \'); ' +
  'if [ -n "$t" ] && [ "$t" != "?" ]; then printf \'%b\' "$SEQ" > "/dev/$t" && exit 0; fi; ' +
  'p=$(ps -o ppid= -p "$p" | tr -d \' \'); done; exit 1'

// Windows has no ptys to walk to: the binary's `term-write` puts the raw bytes (real ESC and BEL) on the console from env PURR_SEQ.
const winPaint = (ground: string) => `\x1b]11;${ground}\x07`
const WIN_RESET = '\x1b]111\x07'

/** The mod has no platform API; Windows sets `OS=Windows_NT` for every process. */
async function isWindows($: Pick<EngineInterface, 'env'>): Promise<boolean> {
  try {
    return (await $.env.get('OS')) === 'Windows_NT'
  } catch {
    return false
  }
}

type Host = Pick<EngineInterface, 'process' | 'env' | 'fs'>

/** True when the sequence reached a terminal; any failure is "no terminal". */
async function send($: Host, ground: Ground): Promise<boolean> {
  try {
    if (await isWindows($)) {
      const bin = await locateBinary($)
      if (bin === null) return false
      const raw = ground === null ? WIN_RESET : winPaint(ground)
      return (await $.process.run([bin, 'term-write'], { env: { PURR_SEQ: raw }, timeoutMs: SEND_MS })).exitCode === 0
    }
    const seq = ground === null ? RESET : paintSeq(ground)
    return (await $.process.run(['sh', '-c', SEND], { env: { SEQ: seq }, timeoutMs: SEND_MS })).exitCode === 0
  } catch {
    return false
  }
}

/** The ground a theme id paints, or null when the theme is not a Neon one. */
function groundOf(theme: unknown): string | null {
  return typeof theme === 'string' && Object.hasOwn(GROUNDS, theme) ? (GROUNDS[theme] ?? null) : null
}

async function activeGround($: Pick<EngineInterface, 'config'>): Promise<string | null> {
  try {
    return groundOf((await $.config.list()).find(r => r.key === 'theme')?.value)
  } catch {
    return null
  }
}

/** Only a background this module set is ever reset. */
let painted = false

async function paint($: Host, ground: string): Promise<void> {
  if (await send($, ground)) painted = true
}

async function reset($: Host): Promise<void> {
  if (!painted) return
  painted = false
  await send($, null)
}

/** Called from the composition root's one `session.start` hook. */
export async function startBackdrop($: Host & Pick<EngineInterface, 'config'>): Promise<void> {
  const ground = await activeGround($)
  if (ground !== null) await paint($, ground)
}

export function registerBackdrop(on: On): void {
  // A failure here must never hold a theme change: it goes through as it was.
  on('config.set', { key: 'theme' }, async ($, e, next) => {
    const r = await next(e)
    if (r.deny !== undefined) return r
    // paint and reset never throw: the change stands.
    const ground = groundOf(e.value)
    await (ground !== null ? paint($, ground) : reset($))
    return r
  }).catch(($, e, next) => next(e))

  // `/clear` ends the session but Claude Code keeps running on the same theme.
  on('session.end', async ($, e, next) => {
    if (e.reason !== 'clear') await reset($)
    return next(e)
  })
}
