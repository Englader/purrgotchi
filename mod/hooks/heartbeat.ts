// The band's heartbeat: `band-<session id>.alive` in the stream's state dir,
// holding the epoch ms, once a second while the cat is really on screen. The
// statusline drops its own cat only while that beat is fresh (3 s), so a
// crashed, collapsed or surveyed band hands the cat back within seconds.
//
// The beat is written only while a blit was accepted in the last 2 s (an
// accepted blit means the Raster is mounted). The id is read on every tick:
// `/clear` swaps it without a `session.start`. An id the mod cannot learn
// (or one unsafe in a file name) falls back to the global `band.alive`.
//
// The same tick reads the context fill: a new mood bucket (<= 40, <= 80,
// > 80) changes the stream's argv, so the band's cat matches the statusline.
import type { EngineInterface } from 'claude-code'

import { B, reconcile } from './band'
import type { BandHost } from './band'
import { debugLog, errText } from './stream'

const FRESH_MS = 2000

/** The `$` a beat runs on: the band's, plus the file and the session. */
export type HeartbeatHost = BandHost & Pick<EngineInterface, 'fs' | 'session'>

/** The statusline's mood buckets by context fill: calm, busy, panic. */
export function moodBucket(pct: number): 0 | 1 | 2 {
  const p = Math.round(pct)
  return p > 80 ? 2 : p > 40 ? 1 : 0
}

/** `[A-Za-z0-9-]{1,64}` (the Rust reader's rule). */
const validId = (id: unknown): id is string => typeof id === 'string' && /^[A-Za-z0-9-]{1,64}$/.test(id)

async function sessionId($: Pick<EngineInterface, 'session'>): Promise<string | null> {
  try {
    const id = await $.session.id()
    return validId(id) ? id : null
  } catch {
    return null
  }
}

/** One tick of `session.start`'s 1 s clock. Never throws. */
export async function beat($: HeartbeatHost): Promise<void> {
  try {
    if (B.stateDir !== null && (await $.clock.now()) - B.lastOkAt <= FRESH_MS) {
      const id = await sessionId($)
      const file = id === null ? 'band.alive' : `band-${id}.alive`
      await $.fs.write(`${B.stateDir}/${file}`, String(await $.clock.now()))
    }
    const pct = (await $.session.usage()).context.percent ?? 0
    if (moodBucket(pct) !== moodBucket(B.ctxPct)) {
      B.ctxPct = pct
      reconcile($)
    }
  } catch (e) {
    debugLog($, `band heartbeat failed: ${errText(e)}`)
  }
}
