// Dev probe: records the distinct props each restyled component renders with.
import type { EngineInterface, Register } from 'claude-code'

const COMPONENTS = ['Spinner', 'TurnDuration', 'PromptHint', 'SessionMode', 'InfoNotice', 'ToolProgress'] as const

/** component -> distinct JSON.stringify(props), `onScreen` removed. */
const SEEN: Record<string, string[]> = {}

function record(component: string, props: unknown): void {
  try {
    const { onScreen: _drop, ...rest } = (props ?? {}) as Record<string, unknown>
    const s = JSON.stringify(rest)
    const list = (SEEN[component] ??= [])
    if (!list.includes(s)) list.push(s)
  } catch {
    // dev only
  }
}

async function flush($: Pick<EngineInterface, 'fs'>, out: string): Promise<void> {
  try {
    await $.fs.write(out, JSON.stringify(SEEN, null, 1))
  } catch {
    // dev only
  }
}

export const register: Register = on => {
  for (const component of COMPONENTS) {
    on('ui.render', { component }, async (_$, e, next) => {
      record(component, e.props)
      return next(e)
    })
  }
  on('session.start', async ($, e, next) => {
    const r = await next(e)
    const out = (await $.env.get('PURR_PROBE_OUT'))?.trim()
    if (out !== undefined && out !== '') $.clock.every(500, () => void flush($, out))
    return r
  })
}
