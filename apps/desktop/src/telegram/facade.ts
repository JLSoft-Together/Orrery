import type { TelegramButton, TelegramEpisodeMessage, TelegramPatch, TelegramRoute, TelegramSetResult, TelegramState } from '@ccm/shared'
import type { DeskRouter } from '../desk/forward'
import type { SendOutcome, TelegramLimitInfo, TelegramNotifier } from './notifier'

export const TELEGRAM_MODULE = 'telegram'

type RunStateSession = { id: string; state: string; configName: string; deviceId: string | null }
type RunFailedSession = { id: string; cwd: string; configName: string; failure: { code: string; detail?: string } | null }

export interface TelegramApi {
  state(): TelegramState | Promise<TelegramState>
  set(patch: TelegramPatch): TelegramSetResult | Promise<TelegramSetResult>
  routeFor(cwd: string): TelegramRoute | null | Promise<TelegramRoute | null>
  report(cwd: string, text: string, file?: string, sessionId?: string, buttons?: TelegramButton[]): Promise<SendOutcome>
}

export class TelegramFacade implements TelegramApi {
  constructor(
    readonly node: TelegramNotifier,
    private readonly desk: () => DeskRouter | null | undefined,
  ) {}

  private router(): DeskRouter | null {
    const d = this.desk()
    return d && d.mode(TELEGRAM_MODULE) === 'rust' ? d : null
  }

  private async rust<T>(channel: string, arg: unknown, fallback: () => T | Promise<T>): Promise<T> {
    const d = this.router()
    if (!d) return fallback()
    const r = await d.invoke(channel, [arg]).catch(() => undefined)
    return r === undefined || r === null ? fallback() : (r as T)
  }

  get rustMode(): boolean {
    return this.router() !== null
  }

  state(): TelegramState | Promise<TelegramState> {
    return this.router() ? this.rust('desk:tg-state', null, () => this.node.state()) : this.node.state()
  }

  set(patch: TelegramPatch): TelegramSetResult | Promise<TelegramSetResult> {
    return this.router() ? this.rust<TelegramSetResult>('desk:tg-set', { patch }, () => ({ ok: false, error: 'failed' })) : this.node.set(patch)
  }

  routeFor(cwd: string): TelegramRoute | null | Promise<TelegramRoute | null> {
    if (!this.router()) return this.node.routeFor(cwd)
    const d = this.router()!
    return d.invoke('desk:tg-route-for', [{ cwd }]).then((r) => (r && typeof r === 'object' ? (r as TelegramRoute) : null), () => null)
  }

  report(cwd: string, text: string, file?: string, sessionId?: string, buttons?: TelegramButton[]): Promise<SendOutcome> {
    const body = { cwd, text, ...(file ? { file } : {}), ...(sessionId ? { from: sessionId } : {}), ...(buttons ? { buttons } : {}) }
    return this.router() ? this.rust<SendOutcome>('desk:tg-report', body, () => ({ ok: false, error: 'network' })) : this.node.report(cwd, text, file, sessionId, buttons)
  }

  handle(e: TelegramEpisodeMessage): Promise<boolean> {
    return this.router() ? this.rust<boolean>('desk:tg-handle', { episode: e }, () => false) : this.node.handle(e)
  }

  alert(kind: 'limit', info?: TelegramLimitInfo): Promise<boolean>
  alert(kind: 'disk', detail?: { label: string; free: string }): Promise<boolean>
  alert(kind: 'limit' | 'disk', detail?: TelegramLimitInfo | { label: string; free: string }): Promise<boolean> {
    if (this.router()) return this.rust<boolean>('desk:tg-alert', { kind, detail: detail ?? {} }, () => false)
    return kind === 'limit' ? this.node.alert('limit', detail as TelegramLimitInfo | undefined) : this.node.alert('disk', detail as { label: string; free: string } | undefined)
  }

  async runState(session: RunStateSession): Promise<void> {
    if (this.router()) await this.rust('desk:tg-run-state', { session }, () => undefined)
    else await this.node.runState(session)
  }

  runFailedReport(session: RunFailedSession): Promise<boolean> {
    return this.router() ? this.rust<boolean>('desk:tg-run-failed', { session }, () => false) : this.node.runFailedReport(session)
  }

  tick(): Promise<boolean> {
    return this.router() ? this.rust<boolean>('desk:tg-tick', null, () => false) : this.node.tick()
  }

  progress(id: number, fraction: number): void {
    void this.router()?.invoke('desk:tg-upload-progress', [{ id, fraction }]).catch(() => undefined)
  }

  stop(): void {
    this.node.stop()
    void this.router()?.invoke('desk:tg-stop', []).catch(() => undefined)
  }
}
