// The TurnDuration line: `◢◤ EXECUTED IN 3.2s ◢◤`. The engine's word is flavour only.
import type { Elements, On, RenderElement } from 'claude-code'

import { NEON, PALETTE, turnText } from './neon'

export type TextCtor = Elements['terminal']['Text']

export function turnDurationTree(Text: TextCtor, durationMs: number): RenderElement {
  return Text({
    wrap: 'truncate-end',
    children: [
      Text({ color: PALETTE.magenta, children: [`${NEON.turnFrame} `] }),
      Text({ color: PALETTE.cyan, bold: true, children: [turnText(durationMs)] }),
      Text({ color: PALETTE.magenta, children: [` ${NEON.turnFrame}`] }),
    ],
  })
}

export function registerTurnDuration(on: On): void {
  on('ui.render', { component: 'TurnDuration' }, ($, e, next) => {
    if (e.surface !== 'terminal') return next(e)
    return turnDurationTree($.ui.resolve(e).Text, e.props.durationMs)
  })
}
