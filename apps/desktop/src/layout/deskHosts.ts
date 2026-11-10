import type { IpcMainInvokeEvent } from 'electron'
import type { HostHandlers } from '../desk/host'
import type { PreviewServer } from '../preview/server'

export interface LayoutHostDeps {
  resolveDir: (p: unknown) => Promise<string | null>
  server: Pick<PreviewServer, 'urlFor'>
  confirm?: (event: IpcMainInvokeEvent, info: { bytes: number; host: string }) => Promise<boolean>
}

const field = (params: unknown, key: string): unknown => (params && typeof params === 'object' ? (params as Record<string, unknown>)[key] : undefined)

export function layoutHosts(deps: LayoutHostDeps, event: IpcMainInvokeEvent | null): HostHandlers {
  return {
    'preview.explorerRoot': (p) => deps.resolveDir(field(p, 'root')),
    'preview.sender': () => event?.sender.id ?? 0,
    'layout.urlFor': async (p) => {
      const roots = field(p, 'roots')
      const target = await deps.server.urlFor(String(field(p, 'file') ?? ''), Array.isArray(roots) ? roots.map(String) : [])
      return target ? { url: target.url } : null
    },
    'layout.confirm': async (p) => {
      if (!event || !deps.confirm) return false
      return (await deps.confirm(event, { bytes: Number(field(p, 'bytes') ?? 0), host: String(field(p, 'host') ?? '') }).catch(() => false)) === true
    },
  }
}
