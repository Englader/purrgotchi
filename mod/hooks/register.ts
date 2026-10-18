// Composition root: the cat band above the prompt.
import type { EngineInterface, Register } from 'claude-code'

import { B, KEY, MAX_COLS, MIN_COLS, RECONCILE_MS, blankCells, pickRows, prepareBand, reconcile, statsJson } from './band'
import { beat } from './heartbeat'
import type { HeartbeatHost } from './heartbeat'
import { registerInfoNotice } from './info-notice'
import { registerPromptHint } from './prompt-hint'
import { registerSessionMode } from './session-mode'
import { registerSpinner } from './spinner'
import { registerToolProgress } from './tool-progress'
import { registerTurnDuration } from './turn-duration'

/** `PURR_BAND_STATS`: the dev counters file, rewritten once a second. */
async function writeStats($: Pick<EngineInterface, 'fs' | 'clock'>, path: string): Promise<void> {
  try {
    await $.fs.write(path, statsJson(await $.clock.now()))
  } catch {
    // dev only
  }
}

async function startBand($: HeartbeatHost & Pick<EngineInterface, 'env' | 'fs' | 'process' | 'clock' | 'ui'>): Promise<void> {
  await prepareBand($)
  $.clock.every(RECONCILE_MS, () => reconcile($))
  $.clock.every(1000, () => void beat($))
  const stats = (await $.env.get('PURR_BAND_STATS'))?.trim()
  if (stats !== undefined && stats !== '') $.clock.every(1000, () => void writeStats($, stats))
  // The band rendered before the options were read: draw it again.
  $.ui.invalidate('ui.render')
}

export const register: Register = on => {
  on('session.start', async ($, e, next) => {
    const r = await next(e)
    await startBand($)
    return r
  })

  // `/clear` keeps the child: session.end does nothing.

  on('ui.render', { component: 'AbovePrompt' }, async ($, e, next) => {
    if (e.surface !== 'terminal') return next(e)
    if (e.props.hasSurvey || B.opts === null || B.bin === null || e.props.bodyColumns < MIN_COLS) {
      B.rows = 0
      return next(e)
    }
    const rows = pickRows(B.opts.rows, e.props.maxRows)
    B.rows = rows
    if (rows === 0) return next(e)
    B.requestId = e.requestId
    B.denyRun = 0 // a render ends a deny pause
    B.cols = Math.min(MAX_COLS, e.props.bodyColumns)
    const cells = B.cells !== null && B.helloCols === B.cols && B.helloRows === rows ? B.cells : blankCells(B.cols, rows)
    const { Raster } = $.ui.resolve(e)
    return Raster({ key: KEY, columns: B.cols, rows, cells })
  })
  registerSpinner(on)
  registerTurnDuration(on)
  registerInfoNotice(on)
  registerSessionMode(on)
  registerPromptHint(on)
  registerToolProgress(on)
}
