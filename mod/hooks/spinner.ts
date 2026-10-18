// The Spinner word: the engine's word re-picked from the Neon list, stable per word.
import type { On } from 'claude-code'

import { spinnerWord } from './neon'

export function registerSpinner(on: On): void {
  on('ui.render', { component: 'Spinner' }, ($, e, next) => {
    if (e.surface !== 'terminal' || e.props.message !== null) return next(e)
    return next({ ...e, props: { ...e.props, word: spinnerWord(e.props.word) } })
  })
}
