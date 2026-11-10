import type { HostHandlers } from '../desk/host'

const field = (params: unknown, key: string): unknown => (params && typeof params === 'object' ? (params as Record<string, unknown>)[key] : undefined)

export interface AndroidGateDeps {
  resolveDir: (dir: unknown) => Promise<string | null>
  resolveFile?: (p: unknown) => Promise<string | null>
}

export function androidGateHosts(deps: AndroidGateDeps): HostHandlers {
  return {
    'gate.dir': (p) => deps.resolveDir(field(p, 'dir')),
    'gate.file': (p) => (deps.resolveFile ? deps.resolveFile(field(p, 'path')) : null),
  }
}
