export const CONTEXT_WARN_DEFAULT = 500_000
export const CONTEXT_WARN_MAX = 2_000_000

export function clampContextWarn(value: number): number {
  if (!Number.isFinite(value)) return CONTEXT_WARN_DEFAULT
  return Math.min(CONTEXT_WARN_MAX, Math.max(0, Math.round(value)))
}

export function contextOverWarn(tokens: number | undefined, threshold: number): boolean {
  return threshold > 0 && tokens !== undefined && Number.isFinite(tokens) && tokens >= threshold
}

export interface ContextSample {
  id: string
  tokens: number | undefined
}

export function contextCrossings(armed: ReadonlySet<string>, samples: readonly ContextSample[], threshold: number): { fire: string[]; armed: Set<string> } {
  const next = new Set<string>()
  const fire: string[] = []
  for (const { id, tokens } of samples) {
    if (tokens === undefined && armed.has(id)) {
      next.add(id)
      continue
    }
    if (!contextOverWarn(tokens, threshold)) continue
    next.add(id)
    if (!armed.has(id)) fire.push(id)
  }
  return { fire, armed: next }
}

export const CONTEXT_WARNED_KEY = 'ccm.contextWarned'
const WARNED_MAX = 200

export function parseWarned(raw: string | null): Set<string> {
  if (!raw) return new Set()
  try {
    const list: unknown = JSON.parse(raw)
    return new Set(Array.isArray(list) ? list.filter((x): x is string => typeof x === 'string').slice(-WARNED_MAX) : [])
  } catch {
    return new Set()
  }
}

export function serializeWarned(armed: ReadonlySet<string>): string {
  return JSON.stringify([...armed].slice(-WARNED_MAX))
}

export function loadWarned(): Set<string> {
  try {
    return parseWarned(localStorage.getItem(CONTEXT_WARNED_KEY))
  } catch {
    return new Set()
  }
}

export function saveWarned(armed: ReadonlySet<string>): void {
  try {
    localStorage.setItem(CONTEXT_WARNED_KEY, serializeWarned(armed))
  } catch {
    return
  }
}
