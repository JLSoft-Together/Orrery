import type { Db } from './idb'

export interface NotifyPrefs {
  waiting: boolean
  done: boolean
  error: boolean
  content: boolean
  bells: Record<string, boolean>
}

export const DEFAULT_NOTIFY: NotifyPrefs = { waiting: true, done: true, error: false, content: false, bells: {} }

const KEY = 'notifyPrefs'

export function normalizeNotify(raw: unknown): NotifyPrefs {
  const o = raw && typeof raw === 'object' ? (raw as Record<string, unknown>) : {}
  const bells: Record<string, boolean> = {}
  if (o.bells && typeof o.bells === 'object') {
    for (const [id, on] of Object.entries(o.bells as Record<string, unknown>)) if (typeof on === 'boolean') bells[id] = on
  }
  return {
    waiting: typeof o.waiting === 'boolean' ? o.waiting : DEFAULT_NOTIFY.waiting,
    done: typeof o.done === 'boolean' ? o.done : DEFAULT_NOTIFY.done,
    error: typeof o.error === 'boolean' ? o.error : DEFAULT_NOTIFY.error,
    content: typeof o.content === 'boolean' ? o.content : DEFAULT_NOTIFY.content,
    bells,
  }
}

export function bellOn(prefs: NotifyPrefs, machineId: string): boolean {
  return prefs.bells[machineId] ?? true
}

export async function loadNotify(db: Db): Promise<NotifyPrefs> {
  return normalizeNotify(await db.get('kv', KEY))
}

export async function saveNotify(db: Db, prefs: NotifyPrefs): Promise<void> {
  await db.put('kv', JSON.parse(JSON.stringify(prefs)), KEY)
}
