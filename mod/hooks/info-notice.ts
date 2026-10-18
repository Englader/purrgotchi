// InfoNotice: the engine's words, uppercased (paths and filenames keep their case), after a `▸`; the `/command` keeps its case.
import type { On, RenderElement } from 'claude-code'

import { NEON, PALETTE, noticeText } from './neon'
import type { TextCtor } from './turn-duration'

export function noticeTree(Text: TextCtor, text: string, command: string | null): RenderElement {
  return Text({
    wrap: 'truncate-end',
    children: [
      Text({ color: PALETTE.magenta, children: [NEON.notice] }),
      Text({ color: PALETTE.cyan, children: [noticeText(text)] }),
      ...(command === null ? [] : [' ', Text({ color: PALETTE.yellow, children: [command] })]),
    ],
  })
}

export function registerInfoNotice(on: On): void {
  on('ui.render', { component: 'InfoNotice' }, ($, e, next) => {
    if (e.surface !== 'terminal' || e.props.text.trim() === '') return next(e)
    return noticeTree($.ui.resolve(e).Text, e.props.text, e.props.command)
  })
}
