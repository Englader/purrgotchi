// The desktop look, wired: set at session start, the backdrop scenes drawn, and `/purr-look` to switch.
import type { EngineInterface, On, PluginOptions } from 'claude-code'

import { IMAGE_FILES, LOOK_NAMES, lookFor, parseBackdrop, parseLook } from './look'
import type { BackdropChoice, LookChoice, LookVariant } from './look'
import { SCENE_ALT, isScene, sceneSvg } from './scenes'

const PLUGIN = 'purrgotchi'
const COMMAND = 'purr-look'

/** The look this session shows; `/purr-look` moves it, a reload reads the options again. */
let current: LookChoice = 'off'

type ThemeHost = Pick<EngineInterface, 'theme'>

async function trySet($: ThemeHost, variant: LookVariant, backdrop: BackdropChoice, image?: string): Promise<boolean> {
  try {
    await $.theme.set(lookFor(variant, backdrop, image))
    return true
  } catch {
    return false
  }
}

/** Sets the look. A picture the app won't take (none there, too big) gives way to the grid scene; never throws. */
async function apply($: ThemeHost, variant: LookVariant, backdrop: BackdropChoice): Promise<void> {
  current = variant
  if (backdrop === 'image') {
    for (const file of IMAGE_FILES) {
      if (await trySet($, variant, 'image', file)) return
    }
    await trySet($, variant, 'grid')
    return
  }
  await trySet($, variant, backdrop)
}

/** Called from the composition root's one `session.start` hook. */
export async function startLook($: Pick<EngineInterface, 'theme' | 'command'>, options: PluginOptions): Promise<void> {
  try {
    await $.command.register({ name: COMMAND, description: 'Desktop look: grid, violet or off', argumentHint: 'grid|violet|off' })
  } catch {
    // the look still applies without its command
  }
  const look = parseLook(options.desktopLook)
  current = look
  // `off` touches nothing: another mod's theme stands.
  if (look !== 'off') await apply($, look, parseBackdrop(options.desktopBackdrop))
}

export function registerLook(on: On, options: PluginOptions): void {
  on('ui.render', { component: 'Backdrop' }, ($, e, next) => {
    if (e.surface !== 'desktop' && e.surface !== 'mobile') return next(e)
    if (e.props.plugin !== PLUGIN || !isScene(e.props.scene) || current === 'off') return next(e)
    const { Svg } = $.ui.resolve(e)
    return Svg({ source: sceneSvg(e.props.scene, current), alt: SCENE_ALT[e.props.scene], isInteractive: true })
  })

  on('command.run', { command: COMMAND }, async ($, e) => {
    const arg = e.args.trim().toLowerCase()
    if (arg === 'grid' || arg === 'violet') {
      await apply($, arg, parseBackdrop(options.desktopBackdrop))
      return { text: `Desktop look: ${LOOK_NAMES[arg]}.` }
    }
    if (arg === 'off') {
      current = 'off'
      try {
        await $.theme.clear()
      } catch {
        // nothing stood to clear
      }
      return { text: 'Desktop look off.' }
    }
    return { text: `Usage: /${COMMAND} grid|violet|off (now: ${current})` }
  })
}
