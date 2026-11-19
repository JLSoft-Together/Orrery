export const HINT_MIN_PANES = 5
export const HINT_MIN_W = 360
export const HINT_MIN_H = 180

export interface PaneSize {
  key: string
  w: number
  h: number
}

export function isCramped(p: Pick<PaneSize, 'w' | 'h'>, minW = HINT_MIN_W, minH = HINT_MIN_H): boolean {
  return p.w < minW || p.h < minH
}

export function splitSuggestion(panes: readonly PaneSize[], opts: { minPanes?: number; minW?: number; minH?: number } = {}): string[] | null {
  const minPanes = opts.minPanes ?? HINT_MIN_PANES
  if (panes.length < minPanes) return null
  if (panes.some((p) => !(p.w > 0) || !(p.h > 0))) return null
  const small = panes.filter((p) => isCramped(p, opts.minW, opts.minH))
  if (!small.length) return null
  if (small.length < panes.length) return small.map((p) => p.key)
  return panes.slice(panes.length - Math.floor(panes.length / 2)).map((p) => p.key)
}

export const SKIP_KEY = 'ccm.next.splitHintSkip'
export const SKIP_MAX = 200

export function parseSkipped(raw: string | null): string[] {
  if (!raw) return []
  try {
    const v: unknown = JSON.parse(raw)
    return Array.isArray(v) ? v.filter((x): x is string => typeof x === 'string').slice(-SKIP_MAX) : []
  } catch {
    return []
  }
}

export function addSkipped(list: readonly string[], key: string): string[] {
  return [...list.filter((k) => k !== key), key].slice(-SKIP_MAX)
}
