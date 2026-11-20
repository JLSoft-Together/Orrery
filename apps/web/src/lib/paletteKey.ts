export interface PaletteKeyEvent {
  key: string
  ctrlKey: boolean
  metaKey: boolean
  altKey: boolean
}

export function isPaletteKey(e: PaletteKeyEvent, inTerminal: boolean, mac: boolean): boolean {
  if (e.altKey || e.key.toLowerCase() !== 'k') return false
  if (mac && e.metaKey && !e.ctrlKey) return true
  return !inTerminal && (e.ctrlKey || e.metaKey)
}
