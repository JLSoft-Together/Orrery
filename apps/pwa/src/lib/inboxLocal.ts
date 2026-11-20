import { ref, type Ref } from 'vue'
import type { InboxRow } from './inboxModel'

export type HideKind = 'seen' | 'snooze'
export type HideMap = Record<string, number | null>

export const SNOOZE_MS = 15 * 60_000
export const SWIPE_MIN = 72
export const SWIPE_EDGE = 24
export const UNDO_MS = 3000

const STORE_KEY = 'ccm.pwa.inboxHidden'

export const hideKey = (row: Pick<InboxRow, 'paneKey' | 'reason' | 'since'>): string => `${row.paneKey}|${row.reason}|${row.since ?? ''}`

export function swipeDecision(startX: number, dx: number, dy: number): HideKind | null {
  if (startX < SWIPE_EDGE) return null
  if (Math.abs(dx) < SWIPE_MIN) return null
  if (Math.abs(dx) < Math.abs(dy) * 1.5) return null
  return dx < 0 ? 'seen' : 'snooze'
}

export function hideRow(map: HideMap, key: string, kind: HideKind, now: number): HideMap {
  return { ...map, [key]: kind === 'seen' ? null : now + SNOOZE_MS }
}

export function unhideRow(map: HideMap, key: string): HideMap {
  const { [key]: _gone, ...rest } = map
  return rest
}

export function isHidden(map: HideMap, key: string, now: number): boolean {
  if (!(key in map)) return false
  const until = map[key]
  return until === null || (until !== undefined && until > now)
}

export function visibleRows(rows: readonly InboxRow[], map: HideMap, now: number): InboxRow[] {
  return rows.filter((r) => !isHidden(map, hideKey(r), now))
}

export function pruneHidden(map: HideMap, rows: readonly InboxRow[], now: number): HideMap {
  const live = new Set(rows.map(hideKey))
  const next: HideMap = {}
  for (const [k, v] of Object.entries(map)) if (live.has(k) && (v === null || v > now)) next[k] = v
  return next
}

function load(): HideMap {
  try {
    const raw = JSON.parse(localStorage.getItem(STORE_KEY) ?? '{}') as unknown
    if (!raw || typeof raw !== 'object') return {}
    const out: HideMap = {}
    for (const [k, v] of Object.entries(raw)) if (v === null || typeof v === 'number') out[k] = v
    return out
  } catch {
    return {}
  }
}

function save(map: HideMap): void {
  try {
    localStorage.setItem(STORE_KEY, JSON.stringify(map))
  } catch {
    return
  }
}

const hidden: Ref<HideMap> = ref(load())

export function useInboxHidden() {
  function hide(row: InboxRow, kind: HideKind, now = Date.now()): void {
    hidden.value = hideRow(hidden.value, hideKey(row), kind, now)
    save(hidden.value)
  }
  function undo(row: InboxRow): void {
    hidden.value = unhideRow(hidden.value, hideKey(row))
    save(hidden.value)
  }
  function prune(rows: readonly InboxRow[], now = Date.now()): void {
    const next = pruneHidden(hidden.value, rows, now)
    if (Object.keys(next).length === Object.keys(hidden.value).length) return
    hidden.value = next
    save(next)
  }
  return { hidden, hide, undo, prune }
}
