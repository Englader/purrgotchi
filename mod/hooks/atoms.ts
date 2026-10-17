// Every `$.state` atom, in one place. Atoms hold view state only; the binary
// owns all durable state. Keys must be declared in types/index.d.ts.
//
// The engine's module scan (2.1.302) needs every atom's `plugin` and `key` as
// string literals (not a shared const), and accepts `update` / `read` only
// on an atom defined in the same file. So the writers (and later the readers)
// live here too, and other modules call them instead of importing atoms.
import { atom, update } from 'claude-code'
import type { StateDollar } from 'claude-code'

import type { BinaryStatus } from '../types'

export const binaryAtom = atom({ plugin: 'purrgotchi', key: 'binary' } as const, { kind: 'unknown' } as BinaryStatus)

export async function writeBinary($: StateDollar, status: BinaryStatus): Promise<void> {
  await update($, binaryAtom, () => status)
}
