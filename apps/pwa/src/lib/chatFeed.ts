import { RemoteCommandError, type JsonMap } from '../remote/types'
import { applyItems, applyPage, emptyLog, parseChatItems, parseChatPage, type ChatLog } from './chatModel'

export type FeedStatus = 'loading' | 'ready' | 'unsupported' | 'error'

export interface FeedState {
  status: FeedStatus
  log: ChatLog
  older: boolean
  error: string | null
}

export interface FeedDeps {
  command(type: string, fields: JsonMap): Promise<JsonMap>
  onChange(state: FeedState): void
}

const codeOf = (error: unknown): string => (error instanceof RemoteCommandError ? error.code : 'internal')

export class ChatFeed {
  readonly paneKey: string
  private readonly deps: FeedDeps
  private state: FeedState = { status: 'loading', log: emptyLog(), older: false, error: null }
  private watching = false
  private stopped = false
  private paused = false

  constructor(paneKey: string, deps: FeedDeps, cached?: ChatLog | null) {
    this.paneKey = paneKey
    this.deps = deps
    if (cached && cached.gen !== null) this.state = { status: 'ready', log: cached, older: false, error: null }
  }

  get isPaused(): boolean {
    return this.paused
  }

  pause(): void {
    if (this.paused || this.stopped) return
    this.paused = true
    this.deps.command('chat.unwatch', { paneKey: this.paneKey }).catch(() => undefined)
  }

  resume(): Promise<void> {
    if (!this.paused || this.stopped) return Promise.resolve()
    this.paused = false
    return this.watch()
  }

  get current(): FeedState {
    return this.state
  }

  private set(patch: Partial<FeedState>): void {
    this.state = { ...this.state, ...patch }
    this.deps.onChange(this.state)
  }

  async watch(): Promise<void> {
    if (this.watching || this.stopped || this.paused) return
    this.watching = true
    try {
      const reply = await this.deps.command('chat.watch', { paneKey: this.paneKey })
      if (this.stopped) return
      const page = parseChatPage(reply)
      if (!page) {
        this.set({ status: 'error', error: 'invalid' })
        return
      }
      this.set({ status: 'ready', log: applyPage(this.state.log, page, false), error: null })
    } catch (error) {
      if (this.stopped) return
      const code = codeOf(error)
      if (code === 'denied' || code === 'proto') this.set({ status: 'unsupported', error: code })
      else if (code === 'offline') this.set({ error: code })
      else this.set({ status: this.state.log.gen === null ? 'error' : this.state.status, error: code })
    } finally {
      this.watching = false
    }
  }

  async loadOlder(): Promise<void> {
    const { log } = this.state
    if (this.state.older || this.stopped || log.gen === null || log.older === null) return
    this.set({ older: true })
    try {
      const reply = await this.deps.command('chat.page', { paneKey: this.paneKey, gen: log.gen, before: log.older })
      if (this.stopped) return
      const page = parseChatPage(reply)
      if (page) this.set({ log: applyPage(this.state.log, page, page.gen === this.state.log.gen) })
    } catch (error) {
      if (!this.stopped) this.set({ error: codeOf(error) })
    } finally {
      if (!this.stopped) this.set({ older: false })
    }
  }

  handle(event: JsonMap): void {
    if (this.stopped || this.paused || event.paneKey !== this.paneKey) return
    if (event.t === 'chat.items' && typeof event.gen === 'string') {
      const result = applyItems(this.state.log, event.gen, parseChatItems(event.items))
      if (result.stale) {
        if (!this.watching) void this.watch()
        return
      }
      if (result.log !== this.state.log) this.set({ log: result.log })
      return
    }
    if (event.t === 'chat.reset') {
      this.set({ log: emptyLog(), status: 'loading' })
      void this.watch()
    }
  }

  stop(): void {
    if (this.stopped) return
    this.stopped = true
    if (this.paused) return
    this.deps.command('chat.unwatch', { paneKey: this.paneKey }).catch(() => undefined)
  }
}
