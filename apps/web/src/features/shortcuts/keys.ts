const MOD_ORDER = ['Ctrl', 'Alt', 'Shift', 'Mod'] as const
export const MODIFIERS: readonly string[] = MOD_ORDER

const NAMED: Record<string, string> = {
  Tab: 'Tab',
  Enter: 'Enter',
  Esc: 'Escape',
  Space: 'Space',
  PgUp: 'PageUp',
  PgDn: 'PageDown',
  Home: 'Home',
  End: 'End',
  Delete: 'Delete',
  Backspace: 'Backspace',
  '`': 'Backquote',
  '-': 'Minus',
  '+': 'Equal',
  '=': 'Equal',
  '.': 'Period',
  ',': 'Comma',
  '/': 'Slash',
  ';': 'Semicolon',
  "'": 'Quote',
  '[': 'BracketLeft',
  ']': 'BracketRight',
  '\\': 'Backslash',
  '←': 'ArrowLeft',
  '→': 'ArrowRight',
  '↑': 'ArrowUp',
  '↓': 'ArrowDown',
}

const CODE_TOKEN: Record<string, string> = {
  Tab: 'Tab',
  Enter: 'Enter',
  Escape: 'Esc',
  Space: 'Space',
  PageUp: 'PgUp',
  PageDown: 'PgDn',
  Home: 'Home',
  End: 'End',
  Delete: 'Delete',
  Backspace: 'Backspace',
  Backquote: '`',
  Minus: '-',
  Equal: '+',
  Period: '.',
  Comma: ',',
  Slash: '/',
  Semicolon: ';',
  Quote: "'",
  BracketLeft: '[',
  BracketRight: ']',
  Backslash: '\\',
  ArrowLeft: '←',
  ArrowRight: '→',
  ArrowUp: '↑',
  ArrowDown: '↓',
}

export function tokenToCode(token: string): string | null {
  if (/^[A-Z]$/.test(token)) return `Key${token}`
  if (/^[0-9]$/.test(token)) return `Digit${token}`
  if (/^F([1-9]|1[0-2])$/.test(token)) return token
  return NAMED[token] ?? null
}

export function codeToToken(code: string): string | null {
  const letter = /^Key([A-Z])$/.exec(code)
  if (letter) return letter[1]!
  const digit = /^Digit([0-9])$/.exec(code)
  if (digit) return digit[1]!
  if (/^F([1-9]|1[0-2])$/.test(code)) return code
  return CODE_TOKEN[code] ?? null
}

export interface Chord {
  mods: string[]
  code: string
}

export function parseChord(keys: readonly string[], mac: boolean): Chord | null {
  const mods = new Set<string>()
  let code: string | null = null
  for (const k of keys) {
    if (MODIFIERS.includes(k)) mods.add(!mac && k === 'Mod' ? 'Ctrl' : k)
    else if (code === null) code = tokenToCode(k)
    else return null
  }
  if (code === null) return null
  return { mods: MOD_ORDER.filter((m) => mods.has(m)), code }
}

export const chordId = (c: Chord): string => `${c.mods.join('+')}:${c.code}`

export function keysId(keys: readonly string[], mac: boolean): string | null {
  const c = parseChord(keys, mac)
  return c ? chordId(c) : null
}

export function chordKeys(c: Chord): string[] {
  return [...c.mods, codeToToken(c.code) ?? c.code]
}

export interface KeyLike {
  code: string
  ctrlKey: boolean
  altKey: boolean
  shiftKey: boolean
  metaKey: boolean
  getModifierState?: (k: string) => boolean
}

const MODIFIER_CODES = /^(Control|Alt|Shift|Meta|OS)(Left|Right)$/

export function chordFromEvent(e: KeyLike, mac: boolean): Chord | null {
  if (MODIFIER_CODES.test(e.code) || !e.code) return null
  if (e.getModifierState?.('AltGraph')) return null
  if (!mac && e.metaKey) return null
  if (!codeToToken(e.code)) return null
  const mods: string[] = []
  if (e.ctrlKey) mods.push('Ctrl')
  if (e.altKey) mods.push('Alt')
  if (e.shiftKey) mods.push('Shift')
  if (mac && e.metaKey) mods.push('Mod')
  return { mods, code: e.code }
}

export const eventChordId = (e: KeyLike, mac: boolean): string | null => {
  const c = chordFromEvent(e, mac)
  return c ? chordId(c) : null
}
