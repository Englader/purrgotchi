// Neon Grid backdrop: the terminal's background goes #0d0a14 (OSC 11) while the theme is on, back to its own (OSC 111) after.
import type { EngineInterface, On } from 'claude-code'

import { PALETTE, THEME_ID } from './neon'

const PAINT = `\\033]11;${PALETTE.ground}\\007`
const RESET = '\\033]111\\007'
/** session.end shares ~1.5 s with every other hook: one short call. */
const SEND_MS = 1000

// The mod has no raw terminal write and a child has no tty: it walks up its parents to the first with one (Claude Code) and writes there.
const SEND =
  'p=$$; while [ "$p" -gt 1 ]; do t=$(ps -o tty= -p "$p" | tr -d \' \'); ' +
  'if [ -n "$t" ] && [ "$t" != "?" ]; then printf \'%b\' "$SEQ" > "/dev/$t" && exit 0; fi; ' +
  'p=$(ps -o ppid= -p "$p" | tr -d \' \'); done; exit 1'

/** True when the sequence reached a terminal; any failure is "no terminal". */
async function send($: Pick<EngineInterface, 'process'>, seq: string): Promise<boolean> {
  try {
    return (await $.process.run(['sh', '-c', SEND], { env: { SEQ: seq }, timeoutMs: SEND_MS })).exitCode === 0
  } catch {
    return false
  }
}

async function isNeon($: Pick<EngineInterface, 'config'>): Promise<boolean> {
  try {
    return (await $.config.list()).find(r => r.key === 'theme')?.value === THEME_ID
  } catch {
    return false
  }
}

/** Only a background this module set is ever reset. */
let painted = false

async function paint($: Pick<EngineInterface, 'process'>): Promise<void> {
  if (await send($, PAINT)) painted = true
}

async function reset($: Pick<EngineInterface, 'process'>): Promise<void> {
  if (!painted) return
  painted = false
  await send($, RESET)
}

/** Called from the composition root's one `session.start` hook. */
export async function startBackdrop($: Pick<EngineInterface, 'process' | 'config'>): Promise<void> {
  if (await isNeon($)) await paint($)
}

export function registerBackdrop(on: On): void {
  // A failure here must never hold a theme change: it goes through as it was.
  on('config.set', { key: 'theme' }, async ($, e, next) => {
    const r = await next(e)
    if (r.deny !== undefined) return r
    try {
      if (e.value === THEME_ID) await paint($)
      else await reset($)
    } catch {
      // the change stands
    }
    return r
  }).catch(($, e, next) => next(e))

  // `/clear` ends the session but Claude Code keeps running on the same theme.
  on('session.end', async ($, e, next) => {
    if (e.reason !== 'clear') await reset($)
    return next(e)
  })
}
