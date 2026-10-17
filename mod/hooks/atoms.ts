// Every `$.state` atom, in one place. Atoms hold view state only; the binary
// owns all durable state. Keys must be declared in types/index.d.ts.
import { atom } from 'claude-code'

import type { BinaryStatus } from '../types'

export const PLUGIN = 'purrgotchi' as const

export const binaryAtom = atom({ plugin: PLUGIN, key: 'binary' } as const, { kind: 'unknown' } as BinaryStatus)
