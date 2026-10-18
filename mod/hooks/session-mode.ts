// SessionMode: every engine label kept, in order, uppercased in `⟦ ⟧` frames.
import type { On, RenderElement } from 'claude-code'

import { NEON, PALETTE } from './neon'
import type { TextCtor } from './turn-duration'

export function modesTree(Text: TextCtor, modes: readonly string[]): RenderElement {
  return Text({
    wrap: 'truncate-end',
    children: modes.flatMap((mode, i) => [
      ...(i === 0 ? [] : [' ']),
      Text({ color: PALETTE.magenta, children: [NEON.mode[0]] }),
      Text({ color: PALETTE.cyan, children: [mode.toUpperCase()] }),
      Text({ color: PALETTE.magenta, children: [NEON.mode[1]] }),
    ]),
  })
}

export function registerSessionMode(on: On): void {
  on('ui.render', { component: 'SessionMode' }, ($, e, next) => {
    if (e.surface !== 'terminal' || e.props.modes.length === 0) return next(e)
    return modesTree($.ui.resolve(e).Text, e.props.modes)
  })
}
