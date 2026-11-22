import { effectiveKeys, overrides } from '../features/shortcuts/bindings'
import { defaultWorkspaceShortcuts } from '../features/shortcuts/defaults'

/**
 * Keys are tokens: `Mod` is the command key (⌘ on macOS, Ctrl elsewhere), `Ctrl` the real Control key (⌃ on macOS),
 * `Alt`, `Shift`, `Enter`; anything else prints as is. `formatKeys` maps them per platform.
 */
export interface Shortcut {
  id: string
  keys: string[]
  group: 'panels' | 'map' | 'help' | 'workspace' | 'view'
}

export const SHORTCUTS: Shortcut[] = [
  { id: 'sessions', keys: ['S'], group: 'panels' },
  { id: 'sessionsPin', keys: ['Shift', 'S'], group: 'panels' },
  { id: 'tabs', keys: ['0', '…', '9'], group: 'map' },
  { id: 'fit', keys: ['F'], group: 'map' },
  { id: 'monitorMode', keys: ['Alt', '1', '/', '2'], group: 'panels' },
  { id: 'cmdPanel', keys: ['`'], group: 'panels' },
  { id: 'esc', keys: ['Esc'], group: 'map' },
  { id: 'mapNav', keys: ['←↑↓→'], group: 'map' },
  { id: 'mapEnter', keys: ['Enter'], group: 'map' },
  { id: 'mapFold', keys: ['Space'], group: 'map' },
  { id: 'mapCopy', keys: ['Mod', 'C'], group: 'map' },
  { id: 'mapFind', keys: ['Mod', 'F'], group: 'map' },
  { id: 'mapMenu', keys: ['Shift', 'F10'], group: 'map' },
  { id: 'hints', keys: ['H'], group: 'help' },
  { id: 'help', keys: ['?'], group: 'help' },
  { id: 'palette', keys: ['Mod', 'K'], group: 'help' },
  { id: 'settings', keys: ['Mod', ','], group: 'help' },
]

export function workspaceShortcuts(mac: boolean): Shortcut[] {
  return defaultWorkspaceShortcuts(mac).map((s) => (s.id in overrides.value ? { ...s, keys: effectiveKeys(s.id, mac) } : s))
}

/** App zoom and code size (useWheelZoom + the main process). */
export function viewShortcuts(mac: boolean): Shortcut[] {
  const group = 'view' as const
  return mac
    ? [
        { id: 'zoomApp', keys: ['Mod', '+', '−', '0'], group },
        { id: 'zoomCode', keys: ['Alt', 'Mod', '+', '−', '0'], group },
        { id: 'zoomWheel', keys: ['Ctrl', 'Scroll'], group },
      ]
    : [
        { id: 'zoomApp', keys: ['Ctrl', '+', '−', '0'], group },
        { id: 'zoomCode', keys: ['Ctrl', 'Alt', '+', '−', '0'], group },
        { id: 'zoomWheel', keys: ['Ctrl', 'Wheel'], group },
      ]
}

const MAC_ORDER = ['Ctrl', 'Alt', 'Shift', 'Mod']
const WIN_ORDER = ['Mod', 'Ctrl', 'Alt', 'Shift']
const MAC_SYMBOL: Record<string, string> = { Ctrl: '⌃', Alt: '⌥', Shift: '⇧', Mod: '⌘', Enter: '↩', Backspace: '⌫' }

/** Key tokens → labels; each run of modifiers is sorted the platform's way (⌃⌥⇧⌘ on macOS, Ctrl Alt Shift elsewhere). */
export function formatKeys(keys: readonly string[], mac: boolean): string[] {
  const order = mac ? MAC_ORDER : WIN_ORDER
  const out: string[] = []
  let mods: string[] = []
  const flush = (): void => {
    const names = [...new Set(mods.map((k) => (!mac && k === 'Mod' ? 'Ctrl' : k)))]
    for (const m of names.sort((a, b) => order.indexOf(a) - order.indexOf(b))) out.push(mac ? MAC_SYMBOL[m]! : m)
    mods = []
  }
  for (const k of keys) {
    if (order.includes(k)) {
      mods.push(k)
      continue
    }
    flush()
    out.push(mac ? (MAC_SYMBOL[k] ?? k) : k)
  }
  flush()
  return out
}

/** One chord as text: "Ctrl+Shift+T", or "⇧⌘T" on macOS. */
export function keyCombo(keys: readonly string[], mac: boolean): string {
  return formatKeys(keys, mac).join(mac ? '' : '+')
}

/** One workspace chord as text. */
export function shortcutText(id: string, mac: boolean): string {
  return keyCombo(workspaceShortcuts(mac).find((s) => s.id === id)?.keys ?? [], mac)
}

export function globalShortcuts(group: Shortcut['group'], mac: boolean): Shortcut[] {
  return SHORTCUTS.filter((s) => s.group === group).map((s) => (s.id in overrides.value ? { ...s, keys: effectiveKeys(s.id, mac) } : s))
}

export function isTyping(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false
  if (target.isContentEditable || target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.tagName === 'SELECT') return true
  // Comboboxes and listboxes use letters for typeahead.
  return !!target.closest('[role="combobox"], [role="listbox"]')
}

export const ASKED_KEYS: readonly string[] = ['Ctrl', 'Alt', 'Q']

export function askedShortcutText(mac: boolean): string {
  return keyCombo(effectiveKeys('wsAsked', mac), mac)
}

export const detachKeys = (mac: boolean): readonly string[] => (mac ? ['Alt', 'Mod', 'T'] : ['Ctrl', 'Alt', 'T'])

export function detachShortcutText(mac: boolean): string {
  return keyCombo(detachKeys(mac), mac)
}
