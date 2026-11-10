import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process'
import { existsSync } from 'node:fs'
import path from 'node:path'
import { createInterface } from 'node:readline'
import type { ActivityEvent, AgentNoteMessage, BackgroundJob, DiskAlertMessage, PlanLimits, ResourceReport, TelegramAsk, TerminalSession } from '@ccm/shared'
import { Toaster, type NotifyPatch, type ToastLocale } from '../src/toast'

const exe = process.platform === 'win32' ? 'toast-script.exe' : 'toast-script'
const target = path.resolve(__dirname, '../../core/target')
export const TOAST_BIN = process.env.CCM_TOAST_SCRIPT?.trim() || [path.join(target, 'debug', exe), path.join(target, 'release', exe)].find((p) => existsSync(p))

export type Cmd = Record<string, unknown> & { cmd: string }
export interface StepOut {
  result: unknown
  out: Record<string, unknown>[]
}

const plain = (v: unknown): unknown => (v === undefined ? null : JSON.parse(JSON.stringify(v)))

export class NodeToast {
  private toaster: Toaster | null = null
  private now = 0
  private snoozed = new Set<string>()
  private out: Record<string, unknown>[] = []

  async run(c: Cmd): Promise<StepOut> {
    this.out = []
    const result = await this.exec(c)
    await new Promise((r) => setImmediate(r))
    return { result: plain(result), out: this.out.map((m) => plain(m) as Record<string, unknown>) }
  }

  private async exec(c: Cmd): Promise<unknown> {
    if (c.cmd === 'init') {
      this.now = c.now as number
      const t = new Toaster(
        { supported: c.supported !== false, port: (c.port as number | undefined) ?? 4317, file: c.file as string | undefined, link: (c.link as string | undefined) ?? null },
        {
          show: async (content) => (this.out.push({ type: 'toast.show', ...content }), true),
          hide: async (tag) => (this.out.push({ type: 'toast.hide', tag }), true),
          now: () => this.now,
        },
      )
      await t.load()
      t.snoozed = (term) => this.snoozed.has(term.id)
      if (c.phone) t.onPhone = (m) => this.out.push({ ...m })
      if (c.telegram) t.onTelegram = (m) => this.out.push({ ...m })
      this.toaster = t
      return t.key
    }
    if (c.cmd === 'advance') {
      this.now += c.ms as number
      return undefined
    }
    if (c.cmd === 'snooze') {
      this.snoozed = new Set(c.ids as string[])
      return undefined
    }
    const t = this.toaster!
    const map = <T>(k: string) => c[k] as Record<string, T> | undefined
    switch (c.cmd) {
      case 'observe': {
        const ctx = map<number>('contextPct')
        const tools = map<string>('toolOf')
        const replies = map<string>('replyOf')
        const asks = map<TelegramAsk>('askOf')
        t.observe({
          terminals: c.terminals as TerminalSession[],
          ...('jobs' in c ? { jobs: c.jobs as BackgroundJob[] } : {}),
          ...('limits' in c ? { limits: c.limits as PlanLimits | null } : {}),
          ...(ctx ? { contextPct: (id: string) => ctx[id] ?? null } : {}),
          ...(tools ? { toolOf: (id: string) => tools[id] } : {}),
          ...(replies ? { replyOf: (id: string) => replies[id] } : {}),
          ...(asks ? { askOf: (id: string) => asks[id] } : {}),
        })
        return undefined
      }
      case 'update':
        return t.update(c.patch as NotifyPatch)
      case 'presence':
        return t.setPresence(c.clientId as number, c.attentive as boolean, c.locale as ToastLocale | undefined)
      case 'drop':
        return t.dropClient(c.clientId as number)
      case 'loops':
        return t.checkLoops(c.events as ActivityEvent[])
      case 'sensitive':
        return t.observeSensitive(c.event as ActivityEvent)
      case 'disk':
        return t.observeDisk(c.msg as DiskAlertMessage)
      case 'note':
        return t.observeNote(c.msg as AgentNoteMessage)
      case 'idle':
        return t.observeIdle(c.report as ResourceReport)
      case 'hideFor':
        return t.hideFor(c.id as string)
      case 'clear':
        return t.clear()
      case 'test':
        return t.test()
      case 'settings':
        return t.settings()
      case 'state':
        return { state: t.state, idleWatch: t.idleWatch, attentive: t.attentive, locale: t.locale }
      case 'accepts':
        return t.accepts((c.key as string | undefined) ?? null)
      case 'link':
        return t.linkFor(c.id as string | undefined)
      case 'snoozeLink':
        return t.snoozeLink(c.id as string)
      default:
        throw new Error(`unknown cmd ${c.cmd}`)
    }
  }
}

export class RustToast {
  private readonly child: ChildProcessWithoutNullStreams
  private readonly waiting: ((line: string) => void)[] = []

  constructor(bin: string) {
    this.child = spawn(bin, [], { stdio: ['pipe', 'pipe', 'pipe'], windowsHide: true })
    createInterface({ input: this.child.stdout }).on('line', (l) => this.waiting.shift()?.(l))
    this.child.stderr.on('data', () => undefined)
  }

  run(c: Cmd): Promise<StepOut> {
    return new Promise((resolve, reject) => {
      this.waiting.push((l) => {
        const r = JSON.parse(l) as { ok: boolean; error?: string; result?: unknown; out: Record<string, unknown>[] }
        if (!r.ok) reject(new Error(`toast-script ${c.cmd}: ${r.error}`))
        else resolve({ result: r.result ?? null, out: r.out })
      })
      this.child.stdin.write(`${JSON.stringify(c)}\n`)
    })
  }

  close(): void {
    this.child.stdin.end()
    if (this.child.exitCode === null) this.child.kill()
  }
}

export interface Split {
  episodes: Record<string, unknown>[]
  byTag: Record<string, Record<string, unknown>[]>
}

export function split(out: Record<string, unknown>[]): Split {
  const episodes: Record<string, unknown>[] = []
  const byTag: Record<string, Record<string, unknown>[]> = {}
  for (const m of out) {
    if (m.type === 'phone' || m.type === 'telegram') episodes.push(m)
    else (byTag[m.tag as string] ??= []).push(m)
  }
  return { episodes, byTag }
}
