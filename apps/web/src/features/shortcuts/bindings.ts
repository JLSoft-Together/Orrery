import { ref } from 'vue'
import { defaultWorkspaceShortcuts, GLOBAL_DEFAULTS } from './defaults'
import { chordFromEvent, chordId, keysId, parseChord, type KeyLike } from './keys'

export interface CommandAction {
  t: string
  dir?: 'row' | 'col'
  step?: 1 | -1
}

export const COMMANDS: Record<string, CommandAction> = {
  wsNew: { t: 'newTab' },
  wsSplit: { t: 'split' },
  wsSplitRight: { t: 'split', dir: 'row' },
  wsSplitDown: { t: 'split', dir: 'col' },
  wsClose: { t: 'closePane' },
  wsReopen: { t: 'reopen' },
  wsZoom: { t: 'zoom' },
  wsAttention: { t: 'attention' },
  wsAsked: { t: 'asked' },
  wsMru: { t: 'mru', step: 1 },
  wsQuickOpen: { t: 'quickOpen' },
  wsExplorer: { t: 'explorer' },
  wsSearch: { t: 'search' },
  wsTermSearch: { t: 'termSearch' },
  wsFocusMode: { t: 'focusMode' },
  wsPrompts: { t: 'prompts' },
  wsRecent: { t: 'recent' },
  wsNotes: { t: 'notes' },
  wsShareMode: { t: 'shareMode' },
  wsDropTerm: { t: 'dropTerm' },
  wsGroupAdd: { t: 'addGroup' },
  wsNavBack: { t: 'navBack' },
  wsNavForward: { t: 'navForward' },
}

export const REBINDABLE: readonly string[] = [...Object.keys(COMMANDS), 'palette']

export type ReservedReason = 'incomplete' | 'noModifier' | 'terminal' | 'system'

const STORE_KEY = 'ccm.shortcuts'

type Overrides = Record<string, string[]>

function load(): Overrides {
  try {
    const raw: unknown = JSON.parse(localStorage.getItem(STORE_KEY) ?? '{}')
    if (!raw || typeof raw !== 'object' || Array.isArray(raw)) return {}
    const out: Overrides = {}
    for (const [id, keys] of Object.entries(raw as Record<string, unknown>)) {
      if (!REBINDABLE.includes(id) || !Array.isArray(keys) || !keys.every((k) => typeof k === 'string')) continue
      if (parseChord(keys as string[], false)) out[id] = keys as string[]
    }
    return out
  } catch {
    return {}
  }
}

export const overrides = ref<Overrides>(load())
export const recording = ref(false)

function persist(): void {
  try {
    if (Object.keys(overrides.value).length) localStorage.setItem(STORE_KEY, JSON.stringify(overrides.value))
    else localStorage.removeItem(STORE_KEY)
  } catch {
    return
  }
}

export function reloadOverrides(): void {
  overrides.value = load()
}

export function defaultKeys(id: string, mac: boolean): string[] {
  const g = GLOBAL_DEFAULTS[id]
  if (g) return [...g]
  return [...(defaultWorkspaceShortcuts(mac).find((s) => s.id === id)?.keys ?? [])]
}

export function effectiveKeys(id: string, mac: boolean): string[] {
  return [...(overrides.value[id] ?? defaultKeys(id, mac))]
}

export const isOverridden = (id: string): boolean => id in overrides.value

export function isAvailable(id: string, mac: boolean): boolean {
  return id in GLOBAL_DEFAULTS || defaultKeys(id, mac).length > 0
}

export function availableCommands(mac: boolean): string[] {
  return REBINDABLE.filter((id) => isAvailable(id, mac))
}

const NAMED_ONLY = new Set(['Tab', 'Enter', 'Escape', 'PageUp', 'PageDown', 'Home', 'End', 'Delete', 'Backspace', 'ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'])
const isFn = (code: string): boolean => /^F([1-9]|1[0-2])$/.test(code)

export function reservedReason(keys: readonly string[], mac: boolean): ReservedReason | null {
  const c = parseChord(keys, mac)
  if (!c) return 'incomplete'
  const has = (m: string): boolean => c.mods.includes(m)
  const ctrl = has('Ctrl')
  const alt = has('Alt')
  const shift = has('Shift')
  const mod = has('Mod')
  const code = c.code
  if (!c.mods.length) return isFn(code) ? null : 'noModifier'
  if (!isFn(code) && c.mods.length === 1 && shift) return 'noModifier'
  const named = NAMED_ONLY.has(code) || isFn(code)
  if (mac) {
    if (!named && c.mods.length === 1 && (ctrl || alt)) return 'terminal'
    if (mod && !alt && !ctrl && !shift && ['KeyC', 'KeyV', 'KeyX', 'KeyA', 'KeyQ', 'KeyH', 'KeyM', 'Tab', 'Space', 'Equal', 'Minus', 'Digit0'].includes(code)) return 'system'
    if (mod && shift && !alt && !ctrl && code === 'Equal') return 'system'
    return null
  }
  if (!named && c.mods.length === 1 && (ctrl || alt)) return 'terminal'
  if (ctrl && shift && !alt && (code === 'KeyC' || code === 'KeyV')) return 'terminal'
  if (alt && !ctrl && !shift && (code === 'Tab' || code === 'F4' || code === 'Escape')) return 'system'
  if (ctrl && !alt && !shift && (code === 'Escape' || code === 'Equal' || code === 'Minus' || code === 'Digit0')) return 'system'
  if (ctrl && shift && !alt && (code === 'Escape' || code === 'Equal')) return 'system'
  if (ctrl && alt && code === 'Delete') return 'system'
  return null
}

function singleChordId(keys: readonly string[], mac: boolean): string | null {
  if (keys.filter((k) => !['Ctrl', 'Alt', 'Shift', 'Mod'].includes(k)).length !== 1) return null
  return keysId(keys, mac)
}

const withShift = (keys: readonly string[]): string[] => [...keys, 'Shift']

export function findConflict(id: string, keys: readonly string[], mac: boolean): string | null {
  const want = keysId(keys, mac)
  if (!want) return null
  const candidates = new Set<string>([...defaultWorkspaceShortcuts(mac).map((s) => s.id), ...Object.keys(GLOBAL_DEFAULTS)])
  for (const other of candidates) {
    if (other === id) continue
    const eff = effectiveKeys(other, mac)
    if (singleChordId(eff, mac) === want) return other
    if (other === 'wsMru' && eff.length && !eff.includes('Shift') && keysId(withShift(eff), mac) === want) return other
  }
  return null
}

export function conflictForMru(keys: readonly string[], mac: boolean): string | null {
  if (keys.includes('Shift')) return null
  const rev = keysId(withShift(keys), mac)
  if (!rev) return null
  for (const other of availableCommands(mac)) {
    if (other === 'wsMru') continue
    if (singleChordId(effectiveKeys(other, mac), mac) === rev) return other
  }
  return null
}

export type SetResult = { ok: true } | { ok: false; reason: ReservedReason } | { ok: false; reason: 'conflict'; with: string }

export function validateBinding(id: string, keys: readonly string[], mac: boolean): SetResult {
  const reserved = reservedReason(keys, mac)
  if (reserved) return { ok: false, reason: reserved }
  const clash = findConflict(id, keys, mac) ?? (id === 'wsMru' ? conflictForMru(keys, mac) : null)
  if (clash) return { ok: false, reason: 'conflict', with: clash }
  return { ok: true }
}

export function setBinding(id: string, keys: readonly string[], mac: boolean): SetResult {
  if (!REBINDABLE.includes(id)) return { ok: false, reason: 'incomplete' }
  const res = validateBinding(id, keys, mac)
  if (!res.ok) return res
  if (keysId(keys, mac) === keysId(defaultKeys(id, mac), mac)) {
    resetBinding(id)
    return res
  }
  overrides.value = { ...overrides.value, [id]: [...keys] }
  persist()
  return res
}

export function resetBinding(id: string): void {
  if (!(id in overrides.value)) return
  const next = { ...overrides.value }
  delete next[id]
  overrides.value = next
  persist()
}

export function resetAllBindings(): void {
  overrides.value = {}
  persist()
}

export function commandOf(action: { t: string; dir?: string }): string | null {
  if (action.t === 'mru') return 'wsMru'
  for (const [id, a] of Object.entries(COMMANDS)) if (a.t === action.t && a.dir === action.dir) return id
  return null
}

export function isRebound(action: { t: string; dir?: string }, mac: boolean): boolean {
  const id = commandOf(action)
  if (!id || !(id in overrides.value)) return false
  return keysId(overrides.value[id]!, mac) !== keysId(defaultKeys(id, mac), mac)
}

export function overrideAction(e: KeyLike, mac: boolean): CommandAction | null {
  const ids = Object.keys(overrides.value)
  if (!ids.length) return null
  const chord = chordFromEvent(e, mac)
  if (!chord) return null
  const got = chordId(chord)
  for (const id of ids) {
    const action = COMMANDS[id]
    if (!action) continue
    if (keysId(overrides.value[id]!, mac) === got) return { ...action }
    if (id === 'wsMru' && !overrides.value[id]!.includes('Shift') && keysId(withShift(overrides.value[id]!), mac) === got) return { t: 'mru', step: -1 }
  }
  return null
}

export function overridePalette(e: KeyLike, mac: boolean): boolean | null {
  const keys = overrides.value.palette
  if (!keys) return null
  return keysId(keys, mac) === (chordFromEvent(e, mac) ? chordId(chordFromEvent(e, mac)!) : null)
}
