import type { IpcMain, IpcMainInvokeEvent } from 'electron'
import { DeskUnavailable, type HostHandlers } from './host'

export type DeskMode = 'rust' | 'node'

export const DESK_MODULES = ['backup', 'buildClean', 'libs'] as const

export const DESK_EVENTS: ReadonlySet<string> = new Set(['ccm:build-event'])

export interface DeskClient {
  readonly dead: string | null
  call(channel: string, args: unknown[], hosts: HostHandlers): Promise<unknown>
}

export const deskFlag = (module: string): string => `CCM_DESK_${module.replace(/([a-z0-9])([A-Z])/g, '$1_$2').toUpperCase()}`

export function deskMode(env: NodeJS.ProcessEnv, module: string): DeskMode {
  return env[deskFlag(module)]?.trim().toLowerCase() === 'node' ? 'node' : 'rust'
}

type Handler<E> = (event: E, ...args: unknown[]) => unknown

export interface DeskRouterOptions {
  client: DeskClient | null
  env: NodeJS.ProcessEnv
  onFallback: (module: string, reason: string, flag: string) => void
  log?: (line: string) => void
}

export class DeskRouter {
  private readonly fell = new Map<string, string>()

  constructor(private readonly opts: DeskRouterOptions) {}

  mode(module: string): DeskMode {
    if (!this.opts.client || this.fell.has(module)) return 'node'
    return deskMode(this.opts.env, module)
  }

  lines(modules: readonly string[] = DESK_MODULES): string[] {
    return modules.map((m) => {
      const fell = this.fell.get(m)
      return fell === undefined ? `desk: ${m} ${this.mode(m)}` : `desk: ${m} node (rust desk unusable, fell back: ${fell})`
    })
  }

  private fallBack(module: string, reason: string): void {
    if (this.fell.has(module)) return
    this.fell.set(module, reason)
    this.opts.onFallback(module, reason, deskFlag(module))
  }

  route<E>(module: string, channel: string, gate: (event: E) => boolean, node: Handler<E>, hosts?: (event: E) => HostHandlers): Handler<E> {
    return async (event, ...args) => {
      const client = this.opts.client
      if (!client || this.mode(module) === 'node' || !gate(event)) return node(event, ...args)
      try {
        return await client.call(channel, args, hosts?.(event) ?? {})
      } catch (err) {
        if (!(err instanceof DeskUnavailable)) throw err
        this.opts.log?.(`[ccm] desk ${channel}: ${err.message}; running the node handler`)
        if (client.dead !== null) this.fallBack(module, client.dead)
        return node(event, ...args)
      }
    }
  }
}

export interface DeskBinding {
  router: DeskRouter
  module: string
  hosts?: (event: IpcMainInvokeEvent) => HostHandlers
}

export function deskHandle(ipcMain: IpcMain, gate: (event: IpcMainInvokeEvent) => boolean, binding?: DeskBinding) {
  return (channel: string, handler: Handler<IpcMainInvokeEvent>): void => {
    ipcMain.handle(channel, binding ? binding.router.route(binding.module, channel, gate, handler, binding.hosts) : handler)
  }
}
