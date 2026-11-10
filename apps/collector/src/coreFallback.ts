export type CoreFallbackKind = 'usage' | 'transcript' | 'codex' | 'sidefiles'

export interface CoreFallbackMessage {
  type: 'core-fallback'
  kind: CoreFallbackKind
  reason: string
}

let listener: ((m: CoreFallbackMessage) => void) | null = null
const told = new Set<CoreFallbackKind>()

export function onCoreFallback(fn: ((m: CoreFallbackMessage) => void) | null): void {
  listener = fn
}

export function reportCoreFallback(kind: CoreFallbackKind, reason: string): void {
  if (told.has(kind)) return
  told.add(kind)
  listener?.({ type: 'core-fallback', kind, reason: reason.slice(0, 200) })
}

export function resetCoreFallback(): void {
  told.clear()
}
