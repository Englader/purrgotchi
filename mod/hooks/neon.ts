// Neon Grid: palette, strings and pure text functions. No runtime imports.

export const PALETTE = {
  magenta: '#ea00d9',
  cyan: '#0abdc6',
  yellow: '#f3f31b',
} as const

export const SPINNER_WORDS: readonly string[] = [
  'JACKING IN',
  'DECRYPTING',
  'SPOOFING ICE',
  'REROUTING POWER',
  'OVERCLOCKING',
  'TRACING SIGNAL',
  'COMPILING DAEMONS',
  'SYNCING WETWARE',
  'BREACHING FIREWALL',
  'UPLOADING',
]

export const NEON = {
  spinner: ['▓▒░ ', ' ░▒▓'],
  turnFrame: '◢◤',
  turnLabel: 'EXECUTED IN',
  mode: ['⟦ ', ' ⟧'],
  notice: '▸ ',
  hintGap: '   ',
  background: 'SEND TO BACKGROUND',
} as const

/** 32-bit FNV-1a over UTF-16 code units. */
function fnv1a(s: string): number {
  let h = 0x811c9dc5
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i)
    h = Math.imul(h, 0x01000193) >>> 0
  }
  return h
}

export function spinnerWord(engineWord: string): string {
  return NEON.spinner[0] + SPINNER_WORDS[fnv1a(engineWord) % SPINNER_WORDS.length] + NEON.spinner[1]
}

export function durationText(ms: number): string {
  const t = Number.isNaN(ms) || ms < 0 ? 0 : ms
  if (t < 60_000) return `${(Math.floor(t / 100) / 10).toFixed(1)}s`
  if (t < 3_600_000) return `${Math.floor(t / 60_000)}m ${Math.floor((t % 60_000) / 1000)}s`
  return `${Math.floor(t / 3_600_000)}h ${Math.floor((t % 3_600_000) / 60_000)}m`
}

export function turnText(ms: number): string {
  return `${NEON.turnLabel} ${durationText(ms)}`
}

/** `key` is uppercased and has no brackets. */
export type KeyHint = { key: string; word: string }
/** `re` is sticky with exactly one capture group, the key. */
export type HintRule = { re: RegExp; word: string }

/** Longest-first; every action is a binary literal or in the 2.1.302 corpus. */
export const HINTS: readonly HintRule[] = [
  { re: /(\S+) for shortcuts/y, word: 'SHORTCUTS' },
  { re: /(\S+) for shell mode/y, word: 'SHELL MODE' },
  { re: /(\S+) to interrupt and send/y, word: 'ABORT + SEND' },
  { re: /(\S+) to interrupt/y, word: 'ABORT' },
  { re: /(\S+) to clear/y, word: 'CLEAR' },
  { re: /(\S+) to delete/y, word: 'DELETE' },
  { re: /(\S+) to stop/y, word: 'STOP' },
  { re: /(\S+) to find/y, word: 'FIND' },
  { re: /(\S+) to group/y, word: 'GROUP' },
  { re: /hold (\S+) to speak/y, word: 'HOLD TO SPEAK' },
  { re: /\((\S+) to cycle\)/y, word: 'CYCLE' },
  { re: /(\S+) for agents/y, word: 'AGENTS' },
]

/** Maps `a · b` style engine hints to key/word pairs, or null if any part is unknown. */
export function parseHint(hint: string): KeyHint[] | null {
  const s = hint.trim()
  if (s === '') return null
  const out: KeyHint[] = []
  let i = 0
  while (i < s.length) {
    let hit: { m: RegExpExecArray; word: string } | null = null
    for (const rule of HINTS) {
      rule.re.lastIndex = i
      const m = rule.re.exec(s)
      if (m && (i + m[0].length === s.length || s[i + m[0].length] === ' ')) {
        hit = { m, word: rule.word }
        break
      }
    }
    if (!hit) return null
    out.push({ key: hit.m[1].toUpperCase(), word: hit.word })
    i += hit.m[0].length
    if (s.startsWith(' · ', i)) i += 3
    else if (s[i] === ' ') i += 1
  }
  return out
}

export const BACKGROUND_HINT = /^\((\S+) to run in background\)$/

export function parseBackgroundHint(hint: string): KeyHint | null {
  const m = BACKGROUND_HINT.exec(hint)
  return m ? { key: m[1].toUpperCase(), word: NEON.background } : null
}
