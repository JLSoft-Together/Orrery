import { pathKey } from '../../git/lib/changesView'
import type { TouchIndex } from './agentTouches'

export const MAX_IGNORED = 200
export const TIP_FILES = 3

export interface ClashPeer {
  terminalId: string
  name: string
  at: number
}

export interface Clash {
  fileKey: string
  path: string
  peers: ClashPeer[]
}

export interface IgnoredFile {
  key: string
  ids: string[]
}

export function clashKey(fileKey: string, idA: string, idB: string): string {
  return idA < idB ? `${fileKey}|${idA}|${idB}` : `${fileKey}|${idB}|${idA}`
}

export function clashList(index: TouchIndex): Clash[] {
  const out: Clash[] = []
  for (const [fileKey, info] of index.files) {
    if (info.state !== 'conflict') continue
    const peers = info.touches.filter((t) => t.kind === 'edit').map(({ terminalId, name, at }) => ({ terminalId, name, at }))
    if (peers.length >= 2) out.push({ fileKey, path: info.path, peers })
  }
  return out.sort((a, b) => a.fileKey.localeCompare(b.fileKey))
}

export function clashesByTerminal(list: readonly Clash[]): Map<string, Clash[]> {
  const out = new Map<string, Clash[]>()
  for (const c of list)
    for (const p of c.peers) {
      const cur = out.get(p.terminalId)
      if (cur) cur.push(c)
      else out.set(p.terminalId, [c])
    }
  return out
}

export function pairKeys(c: Clash, turnKey: (id: string) => string): string[] {
  const ids = [...new Set(c.peers.map((p) => p.terminalId))].sort()
  const keys: string[] = []
  for (let i = 0; i < ids.length; i++)
    for (let j = i + 1; j < ids.length; j++) keys.push(`${clashKey(c.fileKey, ids[i]!, ids[j]!)}|${turnKey(ids[i]!)}|${turnKey(ids[j]!)}`)
  return keys
}

export function newClashes(prev: ReadonlySet<string>, next: readonly Clash[], turnKey: (id: string) => string): Clash[] {
  return next.filter((c) => pairKeys(c, turnKey).some((k) => !prev.has(k)))
}

export function othersOf(c: Clash, terminalId: string): ClashPeer[] {
  return c.peers.filter((p) => p.terminalId !== terminalId)
}

export function relPath(path: string, cwd?: string): string {
  const norm = path.replace(/\\/g, '/').replace(/\/+$/, '')
  if (cwd && isUnder(path, cwd)) return norm.slice(cwd.replace(/\\/g, '/').replace(/\/+$/, '').length + 1)
  return norm.slice(norm.lastIndexOf('/') + 1)
}

export function isUnder(path: string, dir: string): boolean {
  const root = pathKey(dir).replace(/\/+$/, '')
  return pathKey(path).startsWith(`${root}/`)
}

export function summarize<T>(items: readonly T[], max = TIP_FILES): { shown: T[]; more: number } {
  return { shown: items.slice(0, max), more: Math.max(0, items.length - max) }
}

export function addIgnored(list: readonly IgnoredFile[], entry: IgnoredFile, max = MAX_IGNORED): IgnoredFile[] {
  const next = [...list.filter((x) => x.key !== entry.key), entry]
  return next.length > max ? next.slice(next.length - max) : next
}

export function removeIgnored(list: readonly IgnoredFile[], key: string): IgnoredFile[] {
  return list.filter((x) => x.key !== key)
}

export function pruneIgnored(list: readonly IgnoredFile[], alive: (id: string) => boolean): IgnoredFile[] {
  return list.filter((x) => x.ids.some(alive))
}

export function parseIgnored(raw: string | null): IgnoredFile[] {
  if (!raw) return []
  try {
    const data: unknown = JSON.parse(raw)
    if (!Array.isArray(data)) return []
    return data
      .filter((x): x is IgnoredFile => !!x && typeof x === 'object' && typeof (x as IgnoredFile).key === 'string' && Array.isArray((x as IgnoredFile).ids))
      .map((x) => ({ key: x.key, ids: x.ids.filter((id): id is string => typeof id === 'string') }))
      .slice(-MAX_IGNORED)
  } catch {
    return []
  }
}
