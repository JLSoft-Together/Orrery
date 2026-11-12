export const ACTIVITY_CAP = 500

export type MonitorSnapshot = Record<string, unknown>

type Item = Record<string, unknown>

const isItem = (value: unknown): value is Item => !!value && typeof value === 'object' && !Array.isArray(value)

function list(snap: MonitorSnapshot, key: string): unknown[] | null {
  const value = snap[key]
  return Array.isArray(value) ? value : null
}

function upsert(snap: MonitorSnapshot, key: string, item: unknown): void {
  if (!isItem(item)) return
  const target = list(snap, key) ?? []
  snap[key] = target
  const at = target.findIndex((e) => isItem(e) && e.id === item.id)
  if (at >= 0) target[at] = item
  else target.push(item)
}

function merge(snap: MonitorSnapshot, key: string, patch: unknown): void {
  if (!isItem(patch)) return
  const target = list(snap, key)
  if (!target) return
  const at = target.findIndex((e) => isItem(e) && e.id === patch.id)
  if (at < 0) return
  target[at] = { ...(target[at] as Item), ...patch }
}

function remove(snap: MonitorSnapshot, key: string, payload: unknown): void {
  if (!isItem(payload)) return
  const target = list(snap, key)
  if (!target) return
  const at = target.findIndex((e) => isItem(e) && e.id === payload.id)
  if (at >= 0) target.splice(at, 1)
}

const COLLECTIONS: Record<string, string> = { terminal: 'terminals', agent: 'agents', link: 'peerLinks' }

const FIELD: Record<string, string> = {
  'favorites.updated': 'favorites',
  'jobs.updated': 'jobs',
  'models.updated': 'modelNames',
  'snooze.updated': 'snoozes',
}

const WHOLE: Record<string, string> = {
  'limits.updated': 'limits',
  'codexLimits.updated': 'codexLimits',
  'response.updated': 'response',
  resources: 'resources',
  'notify.settings': 'notify',
  'sensitive.settings': 'sensitive',
  'usage.reset': 'usage',
}

export function applyMonitorEvent(snap: MonitorSnapshot, raw: unknown): void {
  if (!isItem(raw)) return
  const type = raw.type
  const payload = raw.payload
  if (typeof type !== 'string') return
  const [family, verb] = type.split('.')
  const collection = family && Object.hasOwn(COLLECTIONS, family) ? COLLECTIONS[family] : undefined
  if (collection) {
    if (verb === 'created') upsert(snap, collection, payload)
    else if (verb === 'updated') merge(snap, collection, payload)
    else if (verb === 'removed') remove(snap, collection, payload)
    return
  }
  if (type === 'activity') {
    if (!isItem(payload)) return
    const target = list(snap, 'activity') ?? []
    snap.activity = target
    target.push(payload)
    if (target.length > ACTIVITY_CAP) target.splice(0, target.length - ACTIVITY_CAP)
    return
  }
  if (type === 'sessions.loading') {
    if (isItem(payload)) snap.loading = payload.loading
    return
  }
  if (type === 'usage.updated') {
    const usage = snap.usage
    if (isItem(usage) && isItem(payload)) usage.buckets = payload.buckets
    return
  }
  const field = Object.hasOwn(FIELD, type) ? FIELD[type] : undefined
  if (field) {
    if (isItem(payload)) snap[field] = payload[field]
    return
  }
  const whole = Object.hasOwn(WHOLE, type) ? WHOLE[type] : undefined
  if (whole) snap[whole] = payload
}

export function applyMonitorEvents(snap: MonitorSnapshot, events: unknown[]): void {
  for (const event of events) applyMonitorEvent(snap, event)
}
