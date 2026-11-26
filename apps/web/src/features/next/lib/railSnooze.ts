import { ref } from 'vue'
import { now } from '../../../core/lib/format'
import { clampSnooze, isSnoozedAt, pruneSnoozed, snoozeUntil, type AttnKind } from './railLive'

const STORE_KEY = 'ccm.railSnooze'

interface Saved {
  until: Record<string, number>
  since: Record<string, number>
}

function load(): Saved {
  const empty: Saved = { until: {}, since: {} }
  try {
    const raw = JSON.parse(sessionStorage.getItem(STORE_KEY) ?? 'null') as Partial<Saved> | null
    if (!raw || typeof raw !== 'object') return empty
    const nums = (o: unknown): Record<string, number> => Object.fromEntries(Object.entries(o && typeof o === 'object' ? o : {}).filter((e): e is [string, number] => typeof e[1] === 'number' && Number.isFinite(e[1])).slice(0, 500))
    const until = pruneSnoozed(nums(raw.until), Date.now())
    const since = Object.fromEntries(Object.entries(nums(raw.since)).filter(([k]) => k in until))
    return { until, since }
  } catch {
    return empty
  }
}

const start = load()
const until = ref<Record<string, number>>(start.until)
const since = ref<Record<string, number>>(start.since)
let sinceOf: (key: string) => number | null | undefined = () => undefined

function persist(): void {
  try {
    sessionStorage.setItem(STORE_KEY, JSON.stringify({ until: until.value, since: since.value } satisfies Saved))
  } catch {
    return
  }
}

export function setSnoozeSinceResolver(fn: (key: string) => number | null | undefined): void {
  sinceOf = fn
}

export function isSnoozed(key: string): boolean {
  return isSnoozedAt(until.value, key, now.value)
}

export function snooze(key: string, kind: AttnKind, minutes: number): { minutes: number; undo: () => void } {
  const prev = until.value[key]
  const prevSince = since.value[key]
  const m = clampSnooze(kind, minutes)
  const at = sinceOf(key) ?? undefined
  until.value = { ...pruneSnoozed(until.value, Date.now()), [key]: snoozeUntil(kind, m, Date.now()) }
  const nextSince = { ...since.value }
  if (at === undefined) delete nextSince[key]
  else nextSince[key] = at
  since.value = nextSince
  persist()
  return {
    minutes: m,
    undo: () => {
      const next = { ...until.value }
      const nextSince = { ...since.value }
      if (prev !== undefined && prev > Date.now()) {
        next[key] = prev
        if (prevSince === undefined) delete nextSince[key]
        else nextSince[key] = prevSince
      } else {
        delete next[key]
        delete nextSince[key]
      }
      until.value = next
      since.value = nextSince
      persist()
    },
  }
}

export function clearSnoozed(key: string): boolean {
  if (!(key in until.value)) return false
  const next = { ...until.value }
  const nextSince = { ...since.value }
  delete next[key]
  delete nextSince[key]
  until.value = next
  since.value = nextSince
  persist()
  return true
}

export function forgetSnoozed(alive: (key: string) => boolean): void {
  const next = pruneSnoozed(until.value, Date.now(), alive)
  if (Object.keys(next).length !== Object.keys(until.value).length) {
    until.value = next
    since.value = Object.fromEntries(Object.entries(since.value).filter(([k]) => k in next))
    persist()
  }
}

export function releaseChanged(current: (key: string) => number | null | undefined): void {
  const drop = Object.keys(until.value).filter((k) => {
    const cur = current(k)
    return since.value[k] !== undefined && cur !== undefined && cur !== since.value[k]
  })
  if (!drop.length) return
  const next = { ...until.value }
  const nextSince = { ...since.value }
  for (const k of drop) {
    delete next[k]
    delete nextSince[k]
  }
  until.value = next
  since.value = nextSince
  persist()
}

export function resetSnoozed(): void {
  until.value = {}
  since.value = {}
  persist()
}
