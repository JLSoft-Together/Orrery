import path from 'node:path'
import type { FocusWindowResult, FolderPickResult, LaunchErrorCode, LaunchMode, OpenApp, OpenResult, TerminalApp } from '@ccm/shared'
import { focusTerminalWindow } from '../../collector/src/focus'
import { pickFolder } from '../../collector/src/folder-pick'
import { hasControlChars, LaunchError, launchClaude } from '../../collector/src/launcher'
import { openFolder } from '../../collector/src/opener'

export const HOST_REQUEST = 'host.req'
export const HOST_RESPONSE = 'host.res'

const MIN_PID = 5
const MAX_PID = 0xffffffff
const MAX_DIR = 1024
const OPEN_APPS: readonly string[] = ['explorer', 'vscode']
const LAUNCH_MODES: readonly string[] = ['new', 'continue']
const TERMINAL_APPS: readonly string[] = ['terminal', 'iterm']

export type HostLaunchResult = { ok: true } | { ok: false; error: LaunchErrorCode }
export type HostPickResult = { result: FolderPickResult; dir?: string }

export interface HostHandlers {
  focus(pid: number): Promise<FocusWindowResult>
  open(dir: string, app: OpenApp): Promise<OpenResult>
  launch(dir: string, mode: LaunchMode, app: TerminalApp | undefined): Promise<void>
  pick(): Promise<HostPickResult>
}

export const systemHandlers: HostHandlers = {
  focus: (pid) => focusTerminalWindow(pid),
  open: (dir, app) => openFolder(dir, app),
  launch: (dir, mode, app) => launchClaude(dir, mode, undefined, app),
  pick: () => pickFolder(),
}

export type HostResponse = { type: typeof HOST_RESPONSE; id: number; result: unknown } | { type: typeof HOST_RESPONSE; id: number; error: string }

export function validDir(dir: unknown): dir is string {
  return typeof dir === 'string' && dir.length > 0 && dir.length <= MAX_DIR && !hasControlChars(dir) && path.isAbsolute(dir)
}

export function validPid(pid: unknown): pid is number {
  return typeof pid === 'number' && Number.isInteger(pid) && pid >= MIN_PID && pid <= MAX_PID
}

function launchOutcome(err: unknown): HostLaunchResult {
  if (err instanceof LaunchError) return { ok: false, error: err.code }
  return { ok: false, error: (err as Error | undefined)?.message === 'directory not found' ? 'notFound' : 'failed' }
}

export class HostRpc {
  private focusing: Promise<unknown> = Promise.resolve()
  private picking = false

  constructor(private readonly handlers: HostHandlers = systemHandlers) {}

  async handleLine(line: string): Promise<string | null> {
    let message: unknown
    try {
      message = JSON.parse(line)
    } catch {
      return null
    }
    const response = await this.handle(message)
    return response ? `${JSON.stringify(response)}\n` : null
  }

  async handle(message: unknown): Promise<HostResponse | null> {
    if (typeof message !== 'object' || message === null) return null
    const m = message as Record<string, unknown>
    if (m.type !== HOST_REQUEST) return null
    const id = m.id
    if (typeof id !== 'number' || !Number.isSafeInteger(id) || id < 0) return null
    const params = typeof m.params === 'object' && m.params !== null && !Array.isArray(m.params) ? (m.params as Record<string, unknown>) : {}
    try {
      const result = await this.dispatch(m.method, params)
      return result === undefined ? { type: HOST_RESPONSE, id, error: 'bad request' } : { type: HOST_RESPONSE, id, result }
    } catch {
      return { type: HOST_RESPONSE, id, error: 'failed' }
    }
  }

  private async dispatch(method: unknown, p: Record<string, unknown>): Promise<unknown> {
    switch (method) {
      case 'focus':
        return validPid(p.pid) ? this.focus(p.pid) : undefined
      case 'open':
        return validDir(p.dir) && typeof p.app === 'string' && OPEN_APPS.includes(p.app) ? this.open(p.dir, p.app as OpenApp) : undefined
      case 'launch': {
        if (!validDir(p.dir) || typeof p.mode !== 'string' || !LAUNCH_MODES.includes(p.mode)) return undefined
        if (p.app !== undefined && (typeof p.app !== 'string' || !TERMINAL_APPS.includes(p.app))) return undefined
        return this.launch(p.dir, p.mode as LaunchMode, p.app as TerminalApp | undefined)
      }
      case 'pickFolder':
        return this.pick()
      default:
        return undefined
    }
  }

  private focus(pid: number): Promise<{ result: FocusWindowResult }> {
    const run = async (): Promise<{ result: FocusWindowResult }> => ({ result: await this.handlers.focus(pid).catch((): FocusWindowResult => 'failed') })
    const next = this.focusing.then(run, run)
    this.focusing = next
    return next
  }

  private async open(dir: string, app: OpenApp): Promise<{ result: OpenResult }> {
    return { result: await this.handlers.open(dir, app).catch((): OpenResult => 'failed') }
  }

  private async launch(dir: string, mode: LaunchMode, app: TerminalApp | undefined): Promise<HostLaunchResult> {
    try {
      await this.handlers.launch(dir, mode, app)
      return { ok: true }
    } catch (err) {
      return launchOutcome(err)
    }
  }

  private async pick(): Promise<HostPickResult> {
    if (this.picking) return { result: 'busy' }
    this.picking = true
    try {
      const r = await this.handlers.pick().catch((): HostPickResult => ({ result: 'failed' }))
      if (r.result === 'ok' && !validDir(r.dir)) return { result: 'failed' }
      return r.result === 'ok' ? { result: 'ok', dir: r.dir } : { result: r.result }
    } finally {
      this.picking = false
    }
  }
}
