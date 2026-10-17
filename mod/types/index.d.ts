// Purrgotchi mod types: the binary's Phase 0 `--json` contract and the
// `PluginState` keys every `atom(...)` in hooks/atoms.ts uses.

/** What session start learned about the purrgotchi binary. */
export type BinaryStatus =
  | { kind: 'unknown' }
  | { kind: 'ready'; version: string }
  | { kind: 'missing'; detail: string }
  | { kind: 'outdated'; version: string }

declare module 'claude-code' {
  interface PluginState {
    purrgotchi: {
      binary: BinaryStatus
    }
  }
}
