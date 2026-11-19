import type { CleanItem, CleanNote } from '@ccm/shared'
import type { CleanProvider } from './providers'

export type DockerRun = (args: string[], timeoutMs?: number) => Promise<{ ok: boolean; code?: string; stdout: string }>

export interface DockerDeps {
  run: DockerRun
}

export const DOCKER_PROBE_MS = 3_000
export const DOCKER_DF_MS = 20_000
export const DOCKER_PRUNE_MS = 30 * 60_000

export const DOCKER_KINDS = ['images', 'containers', 'volumes', 'buildCache'] as const
export type DockerKind = (typeof DOCKER_KINDS)[number]

export const DOCKER_PRUNE: Record<DockerKind, readonly string[]> = {
  images: ['image', 'prune', '-a', '-f'],
  containers: ['container', 'prune', '-f'],
  volumes: ['volume', 'prune', '-f'],
  buildCache: ['builder', 'prune', '-f'],
}

const TYPE_KIND: Record<string, DockerKind> = {
  images: 'images',
  containers: 'containers',
  'local volumes': 'volumes',
  'build cache': 'buildCache',
}

const UNIT: Record<string, number> = { b: 1, kb: 1e3, mb: 1e6, gb: 1e9, tb: 1e12, kib: 1024, mib: 1024 ** 2, gib: 1024 ** 3, tib: 1024 ** 4 }

export function dockerBytes(text: unknown): number {
  if (typeof text !== 'string') return 0
  const m = /^\s*([\d.]+)\s*([a-zA-Z]*)/.exec(text)
  if (!m) return 0
  const n = Number(m[1])
  const mul = UNIT[(m[2] || 'b').toLowerCase()] ?? 0
  return Number.isFinite(n) ? Math.round(n * mul) : 0
}

export interface DockerRow {
  kind: DockerKind
  total: number
  active: number
  size: number
  reclaimable: number
}

export function parseDockerDf(stdout: string): DockerRow[] {
  const out: DockerRow[] = []
  for (const line of stdout.split(/\r?\n/)) {
    const s = line.trim()
    if (!s.startsWith('{')) continue
    try {
      const o = JSON.parse(s) as Record<string, unknown>
      const kind = TYPE_KIND[String(o.Type ?? '').toLowerCase()]
      if (!kind) continue
      out.push({ kind, total: Number(o.TotalCount) || 0, active: Number(o.Active) || 0, size: dockerBytes(o.Size), reclaimable: dockerBytes(o.Reclaimable) })
    } catch {
      continue
    }
  }
  return out
}

export async function dockerProbe(deps: DockerDeps): Promise<CleanNote | null> {
  const r = await deps.run(['info', '--format', '{{.ServerVersion}}'], DOCKER_PROBE_MS)
  if (r.ok && r.stdout.trim()) return null
  if (r.code === 'ENOENT') return 'noCli'
  if (r.code === 'ETIMEDOUT') return 'timeout'
  return 'noDaemon'
}

export async function dockerRows(deps: DockerDeps): Promise<DockerRow[] | null> {
  const r = await deps.run(['system', 'df', '--format', '{{json .}}'], DOCKER_DF_MS)
  return r.ok ? parseDockerDf(r.stdout) : null
}

export const dockerPath = (kind: DockerKind): string => `docker:${kind}`

export const dockerProvider: CleanProvider = {
  id: 'docker',
  group: 'dev',
  preselect: false,
  scan: async (ctx) => {
    const deps = ctx.docker
    if (!deps) return []
    const note = await dockerProbe(deps)
    if (note) {
      ctx.note?.(note)
      return []
    }
    const rows = await dockerRows(deps)
    if (!rows) {
      ctx.note?.('noDaemon')
      return []
    }
    const out: CleanItem[] = []
    for (const kind of DOCKER_KINDS) {
      const row = rows.find((r) => r.kind === kind)
      if (!row) continue
      const item: CleanItem = {
        path: dockerPath(kind),
        size: row.reclaimable,
        group: 'dev',
        category: 'docker',
        kind,
        action: 'command',
        files: Math.max(0, row.total - row.active),
        detail: `${row.total}/${row.active}`,
        restore: `docker ${DOCKER_PRUNE[kind].filter((a) => a !== '-f').join(' ')}`,
        ...(kind === 'volumes' ? { warn: true, hold: true } : {}),
      }
      out.push(item)
      ctx.emit([item])
    }
    return out
  },
}
