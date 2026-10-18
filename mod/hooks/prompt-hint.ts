// PromptHint: `[KEY] WORD` pairs, keys in yellow, words dim. Unknown hints pass.
import type { On, RenderElement } from 'claude-code'

import { NEON, PALETTE, parseHint } from './neon'
import type { KeyHint } from './neon'
import type { TextCtor } from './turn-duration'

export function keyHintTree(Text: TextCtor, hints: readonly KeyHint[]): RenderElement {
  return Text({
    wrap: 'truncate-end',
    children: hints.flatMap((h, i) => [
      ...(i === 0 ? [] : [NEON.hintGap]),
      Text({ color: PALETTE.yellow, children: [`[${h.key}]`] }),
      Text({ dimColor: true, children: [` ${h.word}`] }),
    ]),
  })
}

export function registerPromptHint(on: On): void {
  on('ui.render', { component: 'PromptHint' }, ($, e, next) => {
    if (e.surface !== 'terminal' || e.props.tail !== undefined || (e.props.isDraft && e.props.isWorking)) return next(e)
    const hints = parseHint(e.props.hint)
    if (hints === null) return next(e)
    return keyHintTree($.ui.resolve(e).Text, hints)
  })
}
