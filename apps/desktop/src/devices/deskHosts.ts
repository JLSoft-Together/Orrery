import type { HostHandlers } from '../desk/host'
import type { DeviceManager } from './manager'

const field = (params: unknown, key: string): unknown => (params && typeof params === 'object' ? (params as Record<string, unknown>)[key] : undefined)

export interface DevicesStandingDeps {
  manager: () => DeviceManager | null
  onUse: (serial: string) => void
  confirm?: (req: Record<string, unknown>) => Promise<boolean>
  recDir?: () => string
  recKeeps?: (file: string) => boolean
}

export function devicesStandingHosts(deps: DevicesStandingDeps): HostHandlers {
  return {
    'devices.use': (p) => {
      const serial = field(p, 'serial')
      if (typeof serial === 'string' && serial) deps.onUse(serial)
      return true
    },
    'devices.recDir': () => deps.recDir?.() ?? null,
    'devices.recKeeps': (p) => {
      const file = field(p, 'file')
      return typeof file === 'string' && (deps.recKeeps?.(file) ?? false)
    },
    'devices.confirm': async (p) => {
      const kind = field(p, 'kind')
      if (kind !== 'remove' && kind !== 'wipe') return p && typeof p === 'object' ? ((await deps.confirm?.(p as Record<string, unknown>).catch(() => false)) === true) : false
      const manager = deps.manager()
      if (!manager) return false
      if (kind === 'remove') {
        const ids = field(p, 'ids')
        return Array.isArray(ids) && ids.every((i) => typeof i === 'string') ? ((await manager.confirmRemoveIds(ids as string[]).catch(() => false)) === true) : false
      }
      if (kind === 'wipe') return typeof field(p, 'name') === 'string' ? ((await manager.confirmWipeAvd(field(p, 'name') as string).catch(() => false)) === true) : false
      return false
    },
  }
}
