// Composition root: options → runtime → one register* per feature.
import type { Register } from 'claude-code'

import { readOptions } from './options'

export const register: Register = (on, raw) => {
  const options = readOptions(raw) // Task 1.3 builds the runtime from it
  on('session.start', ($, e, next) => next(e))
}
