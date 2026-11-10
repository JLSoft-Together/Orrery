import { isRunLive, type RunConfig, type RunDevice, type RunEvent, type RunResult, type RunSession, type RunStartRequest } from '@ccm/shared'
import type { DeskRouter } from '../desk/forward'
import type { HostHandlers } from '../desk/host'
import { RunManager, type RunManagerDeps } from './sessions'
import type { RunSettings } from './types'

export const RUN_EVENT = 'ccm:run-event'
export const RUN_STATE_EVENT = 'desk:run-state'
export const RUN_SETTINGS_EVENT = 'desk:run-settings'

const MAX_ENDED = 20

type StateListener = (session: RunSession, config: RunConfig, device: RunDevice | null) => void

const isObject = (v: unknown): v is Record<string, unknown> => !!v && typeof v === 'object' && !Array.isArray(v)
const absOrNull = (v: unknown): string | null => (typeof v === 'string' && v !== '' ? v : null)

function cleanSettings(raw: unknown): RunSettings | null {
  if (!isObject(raw)) return null
  const projects: Record<string, string> = {}
  if (isObject(raw.flutterSdkProjects)) for (const [k, v] of Object.entries(raw.flutterSdkProjects)) if (typeof v === 'string') projects[k] = v
  return { flutterSdk: absOrNull(raw.flutterSdk), androidSdk: absOrNull(raw.androidSdk), jdk: absOrNull(raw.jdk), flutterSdkProjects: projects, layoutAutoCompile: raw.layoutAutoCompile !== false }
}

export class RunFacade {
  readonly node: RunManager
  private readonly sessions = new Map<string, RunSession>()
  private settings: RunSettings | null = null
  private readonly listeners = new Set<StateListener>()

  constructor(
    private readonly deps: RunManagerDeps,
    private readonly desk: () => DeskRouter | null | undefined,
  ) {
    this.node = new RunManager(deps)
  }

  private router(): DeskRouter | null {
    const d = this.desk()
    return d && d.mode('run') === 'rust' ? d : null
  }

  private allowed(root: string): HostHandlers {
    return { 'preview.allowed': () => ({ real: root, roots: [] }) }
  }

  loadSettings(): Promise<void> {
    return this.node.loadSettings()
  }

  getSettings(): RunSettings {
    const s = this.router() ? this.settings : null
    return s ? { ...s, flutterSdkProjects: { ...s.flutterSdkProjects } } : this.node.getSettings()
  }

  list(): RunSession[] {
    return this.router() ? [...this.sessions.values()] : this.node.list()
  }

  live(): RunSession[] {
    return this.list().filter((s) => isRunLive(s.state))
  }

  onSessionState(cb: StateListener): () => void {
    const off = this.node.onSessionState(cb)
    this.listeners.add(cb)
    return () => {
      off()
      this.listeners.delete(cb)
    }
  }

  async configs(root: string): Promise<RunConfig[]> {
    const d = this.router()
    if (!d) return this.node.configs(root)
    const r = await d.invoke('ccm:run-configs', [root], this.allowed(root)).catch(() => null)
    return Array.isArray(r) ? (r as RunConfig[]) : []
  }

  async devices(): Promise<RunDevice[]> {
    const d = this.router()
    if (!d) return this.node.devices()
    const r = await d.invoke('ccm:run-devices').catch(() => null)
    return Array.isArray(r) ? (r as RunDevice[]) : []
  }

  async start(req: RunStartRequest): Promise<RunResult> {
    const d = this.router()
    if (!d) return this.node.start(req)
    const r = await d.invoke('ccm:run-start', [req], this.allowed(req.root)).catch(() => null)
    return isObject(r) && typeof r.ok === 'boolean' ? (r as unknown as RunResult) : { ok: false, error: { code: 'spawn' } }
  }

  stopAll(): Promise<void> {
    return this.node.stopAll()
  }

  killAll(): void {
    this.node.killAll()
  }

  ingest(event: string, payload: unknown): boolean {
    if (event === RUN_EVENT) {
      if (!isObject(payload)) return true
      const e = payload as unknown as RunEvent
      if (e.type === 'session' && isObject(e.session) && typeof e.session.id === 'string') this.mirror(e.session)
      this.deps.broadcast(e)
      return true
    }
    if (event === RUN_STATE_EVENT) {
      if (!isObject(payload) || !isObject(payload.session) || !isObject(payload.config)) return true
      const device = isObject(payload.device) ? (payload.device as unknown as RunDevice) : null
      for (const cb of this.listeners) {
        try {
          cb(payload.session as unknown as RunSession, payload.config as unknown as RunConfig, device)
        } catch {}
      }
      return true
    }
    if (event === RUN_SETTINGS_EVENT) {
      this.settings = cleanSettings(payload) ?? this.settings
      return true
    }
    return false
  }

  private mirror(session: RunSession): void {
    const fresh = !this.sessions.has(session.id)
    this.sessions.set(session.id, session)
    if (!fresh) return
    const ended = [...this.sessions.values()].filter((s) => !isRunLive(s.state)).sort((a, b) => (b.endedAt ?? 0) - (a.endedAt ?? 0))
    for (const s of ended.slice(MAX_ENDED)) this.sessions.delete(s.id)
  }
}
