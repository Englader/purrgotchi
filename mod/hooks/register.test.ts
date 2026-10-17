import { expect, test } from 'claude-code/testing'

import { START } from './testkit'

test('session.start passes through', async ($, on) => {
  on('session.start', ($, e) => ({ cwd: e.cwd }))
  on('command.register', ($, e) => ({ value: { command: e.name } }))
  const r = await $.session.start(START)
  expect(r).toEqual({ cwd: '/tmp' })
})
