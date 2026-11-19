import { mkdirSync, readFileSync, renameSync, rmSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { isDiskFreq, type DiskFreq, type DiskSchedule, type DiskSchedulePatch } from '@ccm/shared'

export const SCHEDULE_FILE = 'schedule.json'
export const IDLE_START_S = 600
export const IDLE_PAUSE_S = 60
export const IDLE_RESUME_S = 300
export const RETRY_MS = 3600_000
export const TICK_MS = 60_000
export const PREEMPT_MS = 5000
const FREQ_MS: Record<Exclude<DiskFreq, 'off'>, number> = { daily: 24 * 3600_000, weekly: 7 * 24 * 3600_000 }
const SKEW_MS = 60_000

interface ScheduleFile {
  freq: DiskFreq
  telegram: boolean
  lastAt?: number
  failAt?: number
  attemptAt?: number
}

export interface SchedulerDeps {
  file: string
  now?: () => number
  idleSeconds: () => number
  onBattery: () => boolean
  status: () => Promise<{ indexed: boolean; scanning: boolean; roots: string[] }>
  start: (roots: string[]) => Promise<boolean>
  control: (op: 'pause' | 'resume' | 'cancel') => Promise<boolean>
  onChange?: (state: DiskSchedule) => void
  tickMs?: number
}

const stamp = (v: unknown): number | undefined => (typeof v === 'number' && Number.isFinite(v) && v > 0 ? Math.floor(v) : undefined)

export function readSchedule(file: string): ScheduleFile {
  try {
    const raw = JSON.parse(readFileSync(file, 'utf8')) as Partial<ScheduleFile> | null
    const o = raw && typeof raw === 'object' ? raw : {}
    const lastAt = stamp(o.lastAt)
    const attemptAt = stamp(o.attemptAt)
    const failAt = stamp(o.failAt)
    return {
      freq: isDiskFreq(o.freq) ? o.freq : 'daily',
      telegram: o.telegram !== false,
      ...(lastAt ? { lastAt } : {}),
      ...(failAt ? { failAt } : {}),
      ...(attemptAt ? { attemptAt } : {}),
    }
  } catch {
    return { freq: 'daily', telegram: true }
  }
}

export class Scheduler {
  private data: ScheduleFile
  private timer: NodeJS.Timeout | null = null
  private ticking = false
  private running = false
  private startedAt = 0
  private pausedByUs = false
  private waiters: (() => void)[] = []

  constructor(private readonly deps: SchedulerDeps) {
    this.data = readSchedule(deps.file)
  }

  private get now(): number {
    return (this.deps.now ?? Date.now)()
  }

  get state(): DiskSchedule {
    return {
      freq: this.data.freq,
      telegram: this.data.telegram,
      last: this.last(),
      running: this.running,
      paused: this.running && this.pausedByUs,
    }
  }

  private last(): DiskSchedule['last'] {
    const { lastAt, failAt } = this.data
    if (failAt && (!lastAt || failAt > lastAt)) return { at: failAt, ok: false }
    return lastAt ? { at: lastAt, ok: true } : null
  }

  get telegram(): boolean {
    return this.data.telegram
  }

  get bgRunning(): boolean {
    return this.running
  }

  start(): void {
    this.stop()
    this.timer = setInterval(() => void this.tick(), this.deps.tickMs ?? TICK_MS)
    this.timer.unref?.()
  }

  stop(): void {
    if (this.timer) clearInterval(this.timer)
    this.timer = null
  }

  private save(): void {
    try {
      mkdirSync(path.dirname(this.deps.file), { recursive: true })
      const tmp = `${this.deps.file}.tmp`
      writeFileSync(tmp, JSON.stringify(this.data), 'utf8')
      renameSync(tmp, this.deps.file)
    } catch {
      rmSync(`${this.deps.file}.tmp`, { force: true })
    }
  }

  private emit(): void {
    this.deps.onChange?.(this.state)
  }

  async set(patch: DiskSchedulePatch): Promise<DiskSchedule> {
    if (patch.freq !== undefined && isDiskFreq(patch.freq)) this.data.freq = patch.freq
    if (typeof patch.telegram === 'boolean') this.data.telegram = patch.telegram
    this.save()
    if (this.data.freq === 'off' && this.running) await this.deps.control('cancel').catch(() => false)
    this.emit()
    return this.state
  }

  private due(): boolean {
    if (this.data.freq === 'off') return false
    const every = FREQ_MS[this.data.freq]
    const ok = (t: number | undefined): number => (t !== undefined && t <= this.now + SKEW_MS ? t : -Infinity)
    return this.now - ok(this.data.lastAt) >= every && this.now - ok(this.data.attemptAt) >= RETRY_MS
  }

  async tick(): Promise<void> {
    if (this.ticking) return
    this.ticking = true
    try {
      if (this.running) return await this.watch()
      if (!this.due()) return
      const idle = this.deps.idleSeconds()
      if (idle < IDLE_START_S || this.deps.onBattery()) return
      const st = await this.deps.status().catch(() => null)
      if (!st || !st.indexed || st.scanning || !st.roots.length) return
      this.data.attemptAt = this.now
      this.save()
      this.running = true
      this.startedAt = this.now
      this.pausedByUs = false
      const started = await this.deps.start(st.roots).catch(() => false)
      if (!started) {
        this.running = false
        this.release()
      }
      this.emit()
    } finally {
      this.ticking = false
    }
  }

  private async watch(): Promise<void> {
    const st = await this.deps.status().catch(() => null)
    if (st && !st.scanning && this.now - this.startedAt > 10_000) {
      this.running = false
      this.pausedByUs = false
      this.release()
      this.emit()
      return
    }
    const idle = this.deps.idleSeconds()
    const busy = idle < IDLE_PAUSE_S || this.deps.onBattery()
    if (!this.pausedByUs && busy) {
      if (await this.deps.control('pause').catch(() => false)) {
        this.pausedByUs = true
        this.emit()
      }
    } else if (this.pausedByUs && !busy && idle >= IDLE_RESUME_S) {
      if (await this.deps.control('resume').catch(() => false)) {
        this.pausedByUs = false
        this.emit()
      }
    }
  }

  noteDone(done: { ok: boolean; cancelled?: boolean }): boolean {
    if (!this.running) return false
    this.running = false
    this.pausedByUs = false
    if (done.ok) this.data.lastAt = this.now
    else if (!done.cancelled) this.data.failAt = this.now
    this.save()
    this.release()
    this.emit()
    return true
  }

  async preempt(): Promise<void> {
    if (!this.running) return
    const gone = new Promise<void>((resolve) => this.waiters.push(resolve))
    await this.deps.control('cancel').catch(() => false)
    let timer: NodeJS.Timeout | undefined
    const limit = new Promise<void>((resolve) => (timer = setTimeout(resolve, PREEMPT_MS)))
    await Promise.race([gone, limit]).finally(() => clearTimeout(timer))
  }

  private release(): void {
    const all = this.waiters
    this.waiters = []
    for (const w of all) w()
  }
}
