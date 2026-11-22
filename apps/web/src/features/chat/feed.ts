import { desktop } from '../../lib/desktop'
import { applyItems, applyPage, CHAT_POLL_MS, emptyLog, parseChatPage, parseChatPoll, type ChatLog } from './model'

export interface ChatTarget {
  key: string
  agent: 'claude' | 'codex'
  session: string
  cwd: string
}

export type FeedStatus = 'loading' | 'ready' | 'error'

export interface FeedState {
  status: FeedStatus
  log: ChatLog
  loadingOlder: boolean
}

export type ChatCall = (req: Record<string, unknown>) => Promise<unknown>

interface ChatBridge {
  chatView?: (req: unknown) => Promise<unknown>
}

const bridge = desktop as unknown as ChatBridge | undefined

export const hasChatView = typeof bridge?.chatView === 'function'

export const bridgeCall: ChatCall = (req) => (bridge?.chatView ? bridge.chatView(req) : Promise.reject(new Error('unsupported')))

export function sameTarget(a: ChatTarget | null, b: ChatTarget | null): boolean {
  return !!a && !!b && a.key === b.key && a.agent === b.agent && a.session === b.session && a.cwd === b.cwd
}

export class ChatFeed {
  private target: ChatTarget
  private readonly call: ChatCall
  private readonly onChange: (s: FeedState) => void
  private readonly pollMs: number
  private state: FeedState = { status: 'loading', log: emptyLog(), loadingOlder: false }
  private timer: ReturnType<typeof setTimeout> | null = null
  private running = false
  private epoch = 0
  private busy = false

  constructor(target: ChatTarget, call: ChatCall, onChange: (s: FeedState) => void, pollMs = CHAT_POLL_MS) {
    this.target = target
    this.call = call
    this.onChange = onChange
    this.pollMs = pollMs
  }

  get current(): FeedState {
    return this.state
  }

  private set(patch: Partial<FeedState>): void {
    this.state = { ...this.state, ...patch }
    this.onChange(this.state)
  }

  private base(op: string): Record<string, unknown> {
    const t = this.target
    return { op, key: t.key, agent: t.agent, session: t.session, cwd: t.cwd }
  }

  start(): void {
    if (this.running) return
    this.running = true
    void this.reload()
  }

  stop(): void {
    if (!this.running) return
    this.running = false
    this.epoch++
    this.clear()
    this.call({ op: 'unwatch', key: this.target.key }).catch(() => undefined)
  }

  retarget(next: ChatTarget): void {
    if (sameTarget(this.target, next)) return
    this.target = next
    this.state = { status: 'loading', log: emptyLog(), loadingOlder: false }
    this.onChange(this.state)
    if (this.running) void this.reload()
  }

  private clear(): void {
    if (this.timer) clearTimeout(this.timer)
    this.timer = null
  }

  private schedule(): void {
    this.clear()
    if (this.running) this.timer = setTimeout(() => void this.poll(), this.pollMs)
  }

  async reload(): Promise<void> {
    const epoch = ++this.epoch
    this.clear()
    try {
      const page = parseChatPage(await this.call(this.base('page')))
      if (epoch !== this.epoch) return
      if (!page) {
        this.set({ status: 'error' })
      } else {
        this.set({ status: 'ready', log: applyPage(this.state.log, page, false) })
      }
    } catch {
      if (epoch !== this.epoch) return
      this.set({ status: 'error' })
    }
    this.schedule()
  }

  async poll(): Promise<void> {
    if (!this.running || this.busy) return
    const epoch = this.epoch
    this.busy = true
    try {
      const req = this.base('poll')
      if (this.state.log.gen !== null) req.gen = this.state.log.gen
      const got = parseChatPoll(await this.call(req))
      if (epoch !== this.epoch) return
      if (!got || got.reset) {
        this.busy = false
        await this.reload()
        return
      }
      const { log, stale } = applyItems(this.state.log, got.gen, got.items)
      if (stale) {
        this.busy = false
        await this.reload()
        return
      }
      if (log !== this.state.log) this.set({ log })
    } catch {
      if (epoch !== this.epoch) return
    } finally {
      this.busy = false
    }
    if (epoch === this.epoch) this.schedule()
  }

  async older(): Promise<boolean> {
    const before = this.state.log.older
    if (before === null || this.state.loadingOlder) return false
    const epoch = this.epoch
    this.set({ loadingOlder: true })
    try {
      const req = this.base('page')
      req.before = before
      if (this.state.log.gen !== null) req.gen = this.state.log.gen
      const page = parseChatPage(await this.call(req))
      if (epoch !== this.epoch) return false
      if (!page) {
        this.set({ loadingOlder: false })
        return false
      }
      this.set({ log: applyPage(this.state.log, page, true), loadingOlder: false })
      return true
    } catch {
      if (epoch === this.epoch) this.set({ loadingOlder: false })
      return false
    }
  }
}
