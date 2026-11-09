import { appendFile, rm } from 'node:fs/promises'
import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vitest'
import { createCoreTranscript, type CoreRequestOptions, type CoreTranscriptClient } from '../src/coreTranscript'
import { CoreTracker } from '../src/coreTracker'
import { SessionTracker } from '../src/tracker'
import { createCoreHub } from '../src/trackerSource'
import { assistant, NOW, realFixture, record, use, type RealFixture } from './tracker-parity-corpus'
import { BIN } from './tracker-twin'

let real: CoreTranscriptClient | null = null
const dirs: string[] = []

interface Wrap {
  client: CoreTranscriptClient
  calls: { op: string; extra?: Record<string, unknown>; opts?: CoreRequestOptions }[]
  sends: { op: string; extra?: Record<string, unknown> }[]
  keys: Map<string, string>
}

function wrap(fail: (op: string, extra: Record<string, unknown> | undefined, w: Wrap) => boolean, rewrite?: (extra: Record<string, unknown>) => Record<string, unknown>): Wrap {
  const w: Wrap = { calls: [], sends: [], keys: new Map(), client: null as unknown as CoreTranscriptClient }
  w.client = {
    request: async (op, extra, opts) => {
      w.calls.push({ op, extra, opts })
      if (op === 'tracker.open' && extra) w.keys.set(String(extra.key), String(extra.terminalId))
      if (fail(op, extra, w)) return null
      return real!.request(op, extra && rewrite ? rewrite(extra) : extra, opts)
    },
    send: (op, extra) => {
      w.sends.push({ op, extra })
      return real!.send(op, extra)
    },
    dispose: () => undefined,
  }
  return w
}

const same = (ts: SessionTracker, rust: CoreTracker) => {
  const t = ts.buildTerminal()
  expect(rust.buildTerminal()).toEqual(t)
  expect(rust.buildAgents(t)).toEqual(ts.buildAgents(t))
  expect(rust.toolSpans()).toEqual(ts.toolSpans())
}

const describeCore = BIN ? describe : describe.skip

describeCore('CoreTracker recovery', () => {
  let fx: RealFixture

  beforeAll(() => {
    real = createCoreTranscript({ bin: BIN!, timeoutMs: 60_000 })
  })

  afterAll(async () => {
    real?.dispose()
    for (const d of dirs) await rm(d, { recursive: true, force: true })
  })

  beforeEach(async () => {
    fx = await realFixture()
    dirs.push(fx.projects)
  })

  const pair = (id: string, w: Wrap) => [new SessionTracker(id, record(), fx.projects, {}, () => NOW), new CoreTracker(id, record(), fx.projects, {}, () => NOW, createCoreHub(() => w.client, () => undefined))] as const

  it('never restarts the shared core on a tracker timeout and falls back only the tracker that keeps failing', async () => {
    const w = wrap((op, extra, w) => (op === 'tracker.open' ? extra?.terminalId === '7:a' : w.keys.get(String(extra?.key)) === '7:a') && op.startsWith('tracker.'))
    const [tsA, a] = pair('7:a', w)
    const [tsB, b] = pair('7:b', w)
    await Promise.all([tsA.sync(true), a.sync(true), tsB.sync(true), b.sync(true)])
    expect(a.usingNode).toBe(true)
    expect(b.usingNode).toBe(false)
    same(tsA, a)
    same(tsB, b)
    expect(w.calls.filter((c) => c.op.startsWith('tracker.')).every((c) => c.opts?.restartOnTimeout === false)).toBe(true)
  })

  it('queues the spans a Node fallback finds after the core went away', async () => {
    let broken = false
    const w = wrap(() => broken)
    const [ts, rust] = pair('7:c', w)
    await Promise.all([ts.sync(true), rust.sync(true)])
    await appendFile(fx.transcript, assistant('m9', '2026-10-07T13:45:00.000Z', [use('toolu_q', 'Grep')], 'tool_use'))
    broken = true
    await Promise.all([ts.sync(false), rust.sync(false)])
    expect(rust.usingNode).toBe(true)
    expect(rust.drainSpans().map((s) => s.id)).toEqual(ts.drainSpans().map((s) => s.id))
    same(ts, rust)
  })

  it('keeps the activity of pages read before a failed page', async () => {
    let syncs = 0
    let armed = false
    const w = wrap((op) => op === 'tracker.sync' && armed && ++syncs === 3, (e) => (armed && syncs < 3 ? { ...e, maxBytes: 1500 } : e))
    const [ts, rust] = pair('7:d', w)
    await Promise.all([ts.sync(true), rust.sync(true)])
    let lines = ''
    for (let i = 0; i < 6; i++) lines += assistant(`p${i}`, '2026-10-07T13:46:00.000Z', [use(`toolu_p${i}`, 'Read')], 'tool_use')
    await appendFile(fx.transcript, lines)
    armed = true
    const [a, b] = await Promise.all([ts.sync(false), rust.sync(false)])
    expect(b.length).toBeGreaterThan(0)
    expect(b.length).toBeLessThan(a.length)
    expect(a.slice(0, b.length)).toEqual(b)
    expect(rust.usingNode).toBe(false)
    same(ts, rust)
    expect(rust.drainSpans().map((s) => s.id).sort()).toEqual([...new Set(ts.drainSpans().map((s) => s.id))].sort())
  })

  it('closes its Rust tracker on dispose', async () => {
    const w = wrap(() => false)
    const [, rust] = pair('7:e', w)
    await rust.sync(true)
    const key = [...w.keys.keys()][0]!
    expect(await real!.request('tracker.sync', { key, catchUp: false })).not.toBeNull()
    rust.dispose()
    expect(w.sends).toContainEqual({ op: 'tracker.close', extra: { key } })
    expect(await real!.request('tracker.sync', { key, catchUp: false })).toBeNull()
    expect(await rust.sync(false)).toEqual([])
  })
})
