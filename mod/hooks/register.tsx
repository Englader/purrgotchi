// Composition root: options → runtime → one register* per feature.
import type { Register } from 'claude-code'

import { registerBand } from './band'
import { registerCommands, registerLayoutCommand, registerLayoutName } from './commands'
import { registerFeed } from './feed'
import { registerMeasure } from './measure'
import { gameEnabled, readOptions } from './options'
import { registerOutputs } from './output'
import { registerPane } from './pane'
import { createRuntime } from './runtime'
import { onSessionStart } from './session'
import { registerTurn } from './turn'

export const register: Register = (on, raw) => {
  const options = readOptions(raw)
  // `layout: off` is no game: no runtime and no hook touches the binary, but
  // `/purr-layout` stays so the person can switch back without `/config`.
  const rt = gameEnabled(options) ? createRuntime(options) : null

  // One session.start hook (the engine's module scan allows a single unmatched one).
  on('session.start', async ($, e, next) => {
    // The engine's start first, so our failure can never delay or break it.
    const r = await next(e)
    try {
      if (rt !== null) await onSessionStart($, rt)
      else await registerLayoutName($)
    } catch {
      // observer
    }
    return r
  })

  if (rt === null) {
    registerLayoutCommand(on, options.layout)
    return
  }
  registerCommands(on, rt)
  registerOutputs(on)
  registerFeed(on, rt)
  registerTurn(on, rt)
  registerMeasure(on, rt)
  registerPane(on, rt)
  if (options.layout === 'bottom') registerBand(on)
}
