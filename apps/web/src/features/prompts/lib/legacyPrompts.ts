import { fitsDraft, MAX_ITEMS, normalizeBody, SCOPE_ALL, type ImportRow } from './promptModel'

export const LEGACY_KEY = 'ccm.prompts'
export const MIGRATED_KEY = 'ccm.prompts.coreMigrated'

export function legacyRows(raw: string | null): ImportRow[] {
  if (!raw) return []
  let list: unknown
  try {
    list = JSON.parse(raw)
  } catch {
    return []
  }
  if (!Array.isArray(list)) return []
  const rows: ImportRow[] = []
  for (const r of list) {
    const p = (r && typeof r === 'object' ? r : {}) as Record<string, unknown>
    if (typeof p.name !== 'string' || typeof p.text !== 'string') continue
    const wanted = typeof p.scope === 'string' ? p.scope : SCOPE_ALL
    const row = { title: p.name.trim(), body: normalizeBody(p.text), scope: wanted === 'global' ? SCOPE_ALL : wanted }
    if (fitsDraft(row)) row.scope = SCOPE_ALL
    if (fitsDraft(row) || rows.length >= MAX_ITEMS) continue
    rows.push(row)
  }
  return rows
}

export function readLegacy(): ImportRow[] {
  try {
    if (localStorage.getItem(MIGRATED_KEY) === '1') return []
    return legacyRows(localStorage.getItem(LEGACY_KEY))
  } catch {
    return []
  }
}

export function markMigrated(): void {
  try {
    localStorage.setItem(MIGRATED_KEY, '1')
  } catch {
    return
  }
}
