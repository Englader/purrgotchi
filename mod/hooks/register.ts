// Composition root (stub): the band lands in Task 1.4.
import type { Register } from 'claude-code'

export const register: Register = on => {
  on('session.start', async ($, e, next) => next(e))
}
