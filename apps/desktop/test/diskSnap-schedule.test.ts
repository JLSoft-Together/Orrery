import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { IDLE_PAUSE_S, IDLE_RESUME_S, IDLE_START_S, readSchedule, RETRY_MS, Scheduler, type SchedulerDeps } from '../src/diskMap/schedule'

const dirs: string[] = []
const tmp = (): string => {
  const d = mkdtempSync(path.join(os.tmpdir(), 'disksnap-sch-'))
  dirs.push(d)
  return d
}
afterEach(() => {
  for (const d of dirs.splice(0)) rmSync(d, { recursive: true, force: true })
})

const H = 3600_000
const DAY = 24 * H

interface Rig {
  s: Scheduler
  file: string
  clock: { now: number }
  power: { idle: number; battery: boolean }
  scan: { indexed: boolean; scanning: boolean; roots: string[] }
  calls: string[]
  startOk: { v: boolean }
  changes: unknown[]
}

function rig(seed?: object, over: Partial<SchedulerDeps> = {}): Rig {
  const file = path.join(tmp(), 'schedule.json')
  if (seed) writeFileSync(file, JSON.stringify(seed))
  const clock = { now: 1_800_000_000_000 }
  const power = { idle: 3600, battery: false }
  const scan = { indexed: true, scanning: false, roots: ['C:\\Users\\a'] }
  const calls: string[] = []
  const startOk = { v: true }
  const changes: unknown[] = []
  const s = new Scheduler({
    file,
    now: () => clock.now,
    idleSeconds: () => power.idle,
    onBattery: () => power.battery,
    status: async () => ({ ...scan }),
    start: async (roots) => {
      calls.push(`start:${roots.join('|')}`)
      if (startOk.v) scan.scanning = true
      return startOk.v
    },
    control: async (op) => {
      calls.push(op)
      if (op === 'cancel') scan.scanning = false
      return true
    },
    onChange: (st) => changes.push(st),
    ...over,
  })
  return { s, file, clock, power, scan, calls, startOk, changes }
}

describe('Scheduler', () => {
  it('defaults to daily with Telegram on and no last run', () => {
    const r = rig()
    expect(r.s.state).toEqual({ freq: 'daily', telegram: true, last: null, running: false, paused: false })
  })

  it('never scans when nothing was ever scanned', async () => {
    const r = rig()
    r.scan.indexed = false
    await r.s.tick()
    r.scan.indexed = true
    r.scan.roots = []
    await r.s.tick()
    expect(r.calls).toEqual([])
  })

  it('starts a light scan of the known roots when idle, on AC and due', async () => {
    const r = rig()
    await r.s.tick()
    expect(r.calls).toEqual(['start:C:\\Users\\a'])
    expect(r.s.state.running).toBe(true)
  })

  it('waits for 10 minutes of idle', async () => {
    const r = rig()
    r.power.idle = IDLE_START_S - 1
    await r.s.tick()
    expect(r.calls).toEqual([])
    r.power.idle = IDLE_START_S
    await r.s.tick()
    expect(r.calls).toHaveLength(1)
  })

  it('does not start on battery', async () => {
    const r = rig()
    r.power.battery = true
    await r.s.tick()
    expect(r.calls).toEqual([])
    r.power.battery = false
    await r.s.tick()
    expect(r.calls).toHaveLength(1)
  })

  it('does not start while another scan runs', async () => {
    const r = rig()
    r.scan.scanning = true
    await r.s.tick()
    expect(r.calls).toEqual([])
  })

  it('respects the frequency: daily 24 h, weekly 7 days, off never', async () => {
    const r = rig({ freq: 'daily', lastAt: 1_800_000_000_000 - DAY + 5000 })
    await r.s.tick()
    expect(r.calls).toEqual([])
    r.clock.now += 6000
    await r.s.tick()
    expect(r.calls).toHaveLength(1)

    const w = rig({ freq: 'weekly', lastAt: 1_800_000_000_000 - 3 * DAY })
    await w.s.tick()
    expect(w.calls).toEqual([])
    w.clock.now += 4 * DAY
    await w.s.tick()
    expect(w.calls).toHaveLength(1)

    const off = rig({ freq: 'off' })
    await off.s.tick()
    expect(off.calls).toEqual([])
  })

  it('treats a last run in the future (clock moved back) as never', async () => {
    const r = rig({ freq: 'daily', lastAt: 1_800_000_000_000 + 10 * DAY, attemptAt: 1_800_000_000_000 + 10 * DAY })
    await r.s.tick()
    expect(r.calls).toHaveLength(1)
  })

  it('records the last successful run and stops being due', async () => {
    const r = rig()
    await r.s.tick()
    expect(r.s.noteDone({ ok: true })).toBe(true)
    expect(r.s.state).toMatchObject({ running: false, last: { at: r.clock.now, ok: true } })
    r.scan.scanning = false
    await r.s.tick()
    expect(r.calls).toHaveLength(1)
    expect(JSON.parse(readFileSync(r.file, 'utf8')).lastAt).toBe(r.clock.now)
  })

  it('a failed run shows as failed and retries only after an hour', async () => {
    const r = rig({ freq: 'daily', lastAt: 1_800_000_000_000 - 3 * DAY })
    await r.s.tick()
    r.scan.scanning = false
    expect(r.s.noteDone({ ok: false })).toBe(true)
    expect(r.s.state.last).toEqual({ at: r.clock.now, ok: false })
    await r.s.tick()
    expect(r.calls).toHaveLength(1)
    r.clock.now += RETRY_MS + 1
    await r.s.tick()
    expect(r.calls).toHaveLength(2)
  })

  it('a cancelled run is not shown as failed', async () => {
    const r = rig({ freq: 'daily', lastAt: 1_800_000_000_000 - 3 * DAY })
    await r.s.tick()
    r.s.noteDone({ ok: false, cancelled: true })
    expect(r.s.state.last).toEqual({ at: 1_800_000_000_000 - 3 * DAY, ok: true })
  })

  it('ignores done events of scans it did not start', () => {
    const r = rig()
    expect(r.s.noteDone({ ok: true })).toBe(false)
    expect(r.s.state.last).toBeNull()
  })

  it('pauses when the user returns or the charger goes, resumes when idle again', async () => {
    const r = rig()
    await r.s.tick()
    r.power.idle = IDLE_PAUSE_S - 1
    await r.s.tick()
    expect(r.calls.at(-1)).toBe('pause')
    expect(r.s.state.paused).toBe(true)
    await r.s.tick()
    expect(r.calls.filter((c) => c === 'pause')).toHaveLength(1)
    r.power.idle = IDLE_RESUME_S - 1
    await r.s.tick()
    expect(r.calls.at(-1)).toBe('pause')
    r.power.idle = IDLE_RESUME_S
    await r.s.tick()
    expect(r.calls.at(-1)).toBe('resume')
    expect(r.s.state.paused).toBe(false)
    r.power.battery = true
    await r.s.tick()
    expect(r.calls.at(-1)).toBe('pause')
    r.power.battery = false
    r.power.idle = 5000
    await r.s.tick()
    expect(r.calls.at(-1)).toBe('resume')
  })

  it('does not resume on battery even when idle', async () => {
    const r = rig()
    await r.s.tick()
    r.power.battery = true
    await r.s.tick()
    r.power.idle = 9999
    await r.s.tick()
    expect(r.calls.filter((c) => c === 'resume')).toHaveLength(0)
  })

  it('drops its running flag when the core says no scan is running any more', async () => {
    const r = rig()
    await r.s.tick()
    r.scan.scanning = false
    r.clock.now += 11_000
    await r.s.tick()
    expect(r.s.state.running).toBe(false)
  })

  it('a start that fails leaves nothing running and retries after an hour', async () => {
    const r = rig()
    r.startOk.v = false
    await r.s.tick()
    expect(r.s.state.running).toBe(false)
    await r.s.tick()
    expect(r.calls).toHaveLength(1)
    r.startOk.v = true
    r.clock.now += RETRY_MS + 1
    await r.s.tick()
    expect(r.s.state.running).toBe(true)
  })

  it('two overlapping ticks start one scan', async () => {
    const r = rig()
    await Promise.all([r.s.tick(), r.s.tick(), r.s.tick()])
    expect(r.calls.filter((c) => c.startsWith('start'))).toHaveLength(1)
  })

  it('turning it off cancels a running scan and keeps the setting', async () => {
    const r = rig()
    await r.s.tick()
    const st = await r.s.set({ freq: 'off' })
    expect(st.freq).toBe('off')
    expect(r.calls.at(-1)).toBe('cancel')
    expect(JSON.parse(readFileSync(r.file, 'utf8')).freq).toBe('off')
    expect(r.changes.length).toBeGreaterThan(0)
  })

  it('persists freq and telegram, rejects junk', async () => {
    const r = rig()
    await r.s.set({ freq: 'weekly', telegram: false })
    await r.s.set({ freq: 'bogus' as never, telegram: 'x' as never })
    expect(readSchedule(r.file)).toMatchObject({ freq: 'weekly', telegram: false })
    expect(r.s.telegram).toBe(false)
  })

  it('reads a damaged file as defaults', () => {
    const file = path.join(tmp(), 's.json')
    writeFileSync(file, '{oops')
    expect(readSchedule(file)).toEqual({ freq: 'daily', telegram: true })
    writeFileSync(file, JSON.stringify({ freq: 'hourly', lastAt: 'x', telegram: false }))
    expect(readSchedule(file)).toEqual({ freq: 'daily', telegram: false })
  })

  it('preempt cancels a running background scan and returns once it ended', async () => {
    const r = rig()
    await r.s.tick()
    const p = r.s.preempt()
    await Promise.resolve()
    expect(r.calls.at(-1)).toBe('cancel')
    r.s.noteDone({ ok: false, cancelled: true })
    await p
    expect(r.s.state.running).toBe(false)
  })

  it('preempt returns after its limit when the done event never comes', async () => {
    vi.useFakeTimers()
    try {
      const r = rig()
      await r.s.tick()
      const p = r.s.preempt()
      await vi.advanceTimersByTimeAsync(6000)
      await p
    } finally {
      vi.useRealTimers()
    }
  })

  it('preempt with nothing running does nothing', async () => {
    const r = rig()
    await r.s.preempt()
    expect(r.calls).toEqual([])
  })

  it('the interval timer drives ticks and stop clears it', async () => {
    vi.useFakeTimers()
    try {
      const r = rig(undefined, { tickMs: 1000 })
      r.s.start()
      await vi.advanceTimersByTimeAsync(1500)
      expect(r.calls).toHaveLength(1)
      r.s.stop()
      r.s.noteDone({ ok: true })
      r.clock.now += 3 * DAY
      r.scan.scanning = false
      await vi.advanceTimersByTimeAsync(5000)
      expect(r.calls).toHaveLength(1)
    } finally {
      vi.useRealTimers()
    }
  })
})
