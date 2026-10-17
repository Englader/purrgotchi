import { expect, test } from 'claude-code/testing'

test('session.start passes through', async ($, on) => {
  on('session.start', ($, e) => ({ cwd: e.cwd }))
  on('command.register', ($, e) => ({ value: { command: e.name } }))
  const r = await $.session.start({ cwd: '/tmp', surface: 'terminal', isInteractive: true })
  expect(r).toEqual({ cwd: '/tmp' })
})
