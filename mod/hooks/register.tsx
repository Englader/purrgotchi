// Composition root: options → runtime → one register* per feature.
import type { Register } from 'claude-code'

import { gameEnabled, readOptions } from './options'
import { createRuntime } from './runtime'
import { onSessionStart } from './session'

export const register: Register = (on, raw) => {
  const options = readOptions(raw)
  const rt = createRuntime(options)

  // layout: 'off' means no game: no hook touches the binary.
  if (!gameEnabled(options)) return

  on('session.start', async ($, e, next) => {
    // The engine's start first, so our failure can never delay or break it.
    const r = await next(e)
    try {
      await onSessionStart($, rt)
    } catch {
      // observer
    }
    return r
  })
}
