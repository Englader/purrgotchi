// ToolProgress: the background pill, `[CTRL+B] SEND TO BACKGROUND`, with the engine's own key.
import type { On } from 'claude-code'

import { parseBackgroundHint } from './neon'
import { keyHintTree } from './prompt-hint'

export function registerToolProgress(on: On): void {
  on('ui.render', { component: 'ToolProgress' }, ($, e, next) => {
    if (e.surface !== 'terminal' || e.props.kind !== 'background_hint') return next(e)
    const hint = parseBackgroundHint(e.props.hint)
    if (hint === null) return next(e)
    return keyHintTree($.ui.resolve(e).Text, [hint])
  })
}
