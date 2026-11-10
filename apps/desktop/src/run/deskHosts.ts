import type { IpcMainInvokeEvent } from 'electron'
import type { HostHandlers } from '../desk/host'

const field = (params: unknown, key: string): unknown => (params && typeof params === 'object' ? (params as Record<string, unknown>)[key] : undefined)

export interface RunHostDeps {
  resolveDir: (dir: unknown) => Promise<string | null>
  ownerOf: (event: IpcMainInvokeEvent, key: string) => string
}

export function runHosts(deps: RunHostDeps, event: IpcMainInvokeEvent): HostHandlers {
  return {
    'preview.allowed': async (p) => {
      const real = await deps.resolveDir(field(p, 'path'))
      return real ? { real, roots: [] } : null
    },
    'run.owner': (p) => deps.ownerOf(event, String(field(p, 'key') ?? '')),
  }
}

export interface RunStandingDeps {
  confirm: () => ((code: string, detail?: string) => Promise<boolean>) | null
  onAdbUse: (serial: string) => void
}

export function runStandingHosts(deps: RunStandingDeps): HostHandlers {
  return {
    'run.confirm': async (p) => {
      const code = field(p, 'code')
      const detail = field(p, 'detail')
      const confirm = deps.confirm()
      if (!confirm || typeof code !== 'string') return false
      return (await confirm(code, typeof detail === 'string' ? detail : undefined).catch(() => false)) === true
    },
    'run.adbUse': (p) => {
      const serial = field(p, 'serial')
      if (typeof serial === 'string' && serial) deps.onAdbUse(serial)
      return true
    },
  }
}
