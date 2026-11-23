export const NOTIFY_LEVELS = ['all', 'waiting', 'off'] as const
export type NotifyLevel = (typeof NOTIFY_LEVELS)[number]

export type LevelMap = Readonly<Record<string, NotifyLevel>>

const WAITING_KINDS: ReadonlySet<string> = new Set(['waiting', 'reminder', 'job'])

export const isNotifyLevel = (v: unknown): v is NotifyLevel => typeof v === 'string' && (NOTIFY_LEVELS as readonly string[]).includes(v)

export const rootKey = (path: string): string => path.replace(/\\/g, '/').replace(/\/+$/, '').toLowerCase()

export function levelFor(map: LevelMap, cwd: string | undefined | null): NotifyLevel {
  if (!cwd) return 'all'
  const dir = rootKey(cwd)
  let best = ''
  for (const root of Object.keys(map)) {
    if ((dir === root || dir.startsWith(`${root}/`)) && root.length > best.length) best = root
  }
  return best ? (map[best] ?? 'all') : 'all'
}

export function levelAllows(level: NotifyLevel, kind: string): boolean {
  if (level === 'all') return true
  if (level === 'off') return false
  return WAITING_KINDS.has(kind)
}

export function withLevel(map: LevelMap, root: string, level: NotifyLevel): Record<string, NotifyLevel> {
  const key = rootKey(root)
  const next = { ...map }
  if (level === 'all') delete next[key]
  else next[key] = level
  return next
}

export function parseLevels(raw: string | null): Record<string, NotifyLevel> {
  if (!raw) return {}
  try {
    const v = JSON.parse(raw) as unknown
    if (!v || typeof v !== 'object' || Array.isArray(v)) return {}
    return Object.fromEntries(Object.entries(v).filter((e): e is [string, NotifyLevel] => isNotifyLevel(e[1]) && e[1] !== 'all').slice(0, 500))
  } catch {
    return {}
  }
}
