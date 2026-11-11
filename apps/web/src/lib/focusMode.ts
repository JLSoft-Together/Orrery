export const FOCUS_KEY = 'ccm.workspace.focusTabs'

export function parseFocus(raw: string | null): Set<string> {
  if (!raw) return new Set()
  try {
    const v: unknown = JSON.parse(raw)
    return new Set(Array.isArray(v) ? v.filter((x): x is string => typeof x === 'string') : [])
  } catch {
    return new Set()
  }
}

export function setFocus(set: ReadonlySet<string>, key: string, on: boolean, live: readonly string[]): Set<string> {
  const out = new Set([...set].filter((k) => live.includes(k)))
  if (on && live.includes(key)) out.add(key)
  else out.delete(key)
  return out
}

export const serializeFocus = (set: ReadonlySet<string>): string => JSON.stringify([...set])
