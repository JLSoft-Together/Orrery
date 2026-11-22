import { overridePalette, recording } from '../../shortcuts/bindings'
import type { KeyLike } from '../../shortcuts/keys'

export interface PaletteKeyEvent {
  key: string
  ctrlKey: boolean
  metaKey: boolean
  altKey: boolean
}

export function isPaletteKey(e: PaletteKeyEvent, inTerminal: boolean, mac: boolean): boolean {
  if (recording.value) return false
  const custom = 'code' in e ? overridePalette(e as PaletteKeyEvent & KeyLike, mac) : null
  if (custom !== null) return custom
  if (e.altKey || e.key.toLowerCase() !== 'k') return false
  if (mac && e.metaKey && !e.ctrlKey) return true
  return !inTerminal && (e.ctrlKey || e.metaKey)
}
