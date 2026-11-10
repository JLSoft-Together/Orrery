import { writeFileSync } from 'node:fs'
import type { HostHandlers } from '../desk/host'
import type { Prepared } from './install'
import type { UpdateDeps } from './manager'

export type UpdateHostEnv = Pick<UpdateDeps, 'kind' | 'assetKind' | 'canDownload' | 'arch' | 'prepare' | 'restart' | 'openUrl' | 'reveal'>

const field = (params: unknown, key: string): unknown => (params && typeof params === 'object' ? (params as Record<string, unknown>)[key] : undefined)

export function updateHosts(env: () => UpdateHostEnv | null): HostHandlers {
  let prepared: Prepared | null = null
  const need = (): UpdateHostEnv => {
    const e = env()
    if (!e) throw new Error('update not ready')
    return e
  }
  return {
    'update.env': () => {
      const e = need()
      return { kind: e.kind, assetKind: e.assetKind, canDownload: e.canDownload, arch: e.arch }
    },
    'update.prepare': async (p) => {
      prepared = await need().prepare(String(field(p, 'file') ?? ''))
      return prepared.ok ? { ok: true } : { ok: false, error: prepared.error }
    },
    'update.restart': async (p) => {
      const pending = field(p, 'pending')
      const file = String(field(pending, 'file') ?? '')
      const text = String(field(pending, 'text') ?? '')
      const ready = prepared
      if (!ready || !ready.ok) return false
      const launch = (): void => {
        try {
          writeFileSync(file, text)
        } catch {
        }
        ready.launch()
      }
      return need()
        .restart(launch, String(field(p, 'version') ?? ''))
        .catch(() => false)
    },
    'update.open': (p) => {
      need().openUrl(String(field(p, 'target') ?? ''))
      return true
    },
    'update.reveal': (p) => {
      need().reveal(String(field(p, 'file') ?? ''))
      return true
    },
  }
}
