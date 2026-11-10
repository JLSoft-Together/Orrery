import { mkdir, writeFile } from 'node:fs/promises'
import path from 'node:path'
import { pathToFileURL } from 'node:url'
import { beforeAll, describe, expect, it } from 'vitest'
import { MonitorStore } from '../src/store'
import { monitorScenario, type Trace } from './trace-scenario'

interface CompareLib {
  compareTraces: (a: unknown, b: unknown, options?: { strict?: boolean; limit?: number }) => { diffs: string[]; steps: number }
}

interface ReplayLib {
  replayBinary: () => string | null
  opsDocument: (trace: Trace, ops: unknown[]) => { format: string; header: Record<string, unknown>; ops: Record<string, unknown>[] }
  withSteps: (ops: Record<string, unknown>[], steps: Array<{ name: string; sent?: unknown[]; at: number }>) => Record<string, unknown>[]
  replay: (doc: unknown, bin?: string | null) => { trace: Trace; log: string }
}

interface RecordLib {
  instrumentStore: (target: object, sink: { op: (op: Record<string, unknown>) => void; sends: () => number; wall?: boolean }) => () => void
}

const lib = (name: string) => pathToFileURL(path.resolve(__dirname, `../../../scripts/${name}`)).href
let compare: CompareLib
let replayLib: ReplayLib
let recorder: RecordLib

beforeAll(async () => {
  compare = (await import(lib('trace-compare.mjs'))) as CompareLib
  replayLib = (await import(lib('trace-replay.mjs'))) as ReplayLib
  recorder = (await import(lib('trace-store-record.mjs'))) as RecordLib
})

describe('store op recorder', () => {
  it('records top-level calls once, object identities, flush delivery and activity clocks', () => {
    const store = new MonitorStore({ activityLimit: 5, batchMs: 1000 })
    const ops: Record<string, unknown>[] = []
    let sends = 0
    let listening = false
    store.subscribe(() => {
      if (listening) sends++
    })
    recorder.instrumentStore(store, { op: (op) => ops.push(op), sends: () => sends })
    store.statusLineCommand = 'node bridge.mjs'
    const diff = { files: 1, insertions: 2, deletions: 0, untracked: 0, at: '2027-01-15T08:00:00.000Z' }
    store.upsertTerminal({ id: 't', title: 'A', status: 'idle', diff })
    store.upsertTerminal({ id: 't', title: 'B', status: 'idle', diff })
    store.upsertAgent({ id: 'a', terminalId: 't', role: 'main', status: 'working' })
    store.flush()
    listening = true
    store.removeTerminal('t')
    store.pushActivity({ kind: 'tool.started' })
    store.snapshotMessage()
    const names = ops.map((o) => (o.op === 'call' ? `call:${String(o.name)}` : String(o.op)))
    expect(names).toEqual(['init', 'set', 'call:upsertTerminal', 'call:upsertTerminal', 'call:upsertAgent', 'flush', 'call:removeTerminal', 'call:pushActivity', 'flush', 'snapshot'])
    const refs = ops.filter((o) => o.name === 'upsertTerminal').map((o) => (o.refs as { diff: number }).diff)
    expect(refs[0]).toBe(refs[1])
    expect(ops.filter((o) => o.op === 'flush').map((o) => o.dropped)).toEqual([true, false])
    const activity = ops.find((o) => o.name === 'pushActivity')!
    expect(typeof activity.now).toBe('number')
    expect(activity.at).toBe(new Date(activity.now as number).toISOString())
  })

  it('places step marks by wall time', () => {
    const ops = [{ op: 'init', w: 1 }, { op: 'call', w: 5 }, { op: 'flush', w: 12 }]
    const out = replayLib.withSteps(ops, [
      { name: 'boot', sent: [], at: 2 },
      { name: 'next', sent: [{ type: 'resync' }], at: 10 },
      { name: 'last', at: 20 },
    ])
    expect(out.map((o) => (o.op === 'step' ? `step:${String(o.name)}` : String(o.op)))).toEqual(['init', 'step:boot', 'call', 'step:next', 'flush', 'step:last'])
  })
})

const bin = process.env.CCM_STORE_REPLAY_SKIP === '1' ? null : (await import(lib('trace-replay.mjs')) as ReplayLib).replayBinary()

describe.skipIf(!bin)('Rust core-store replay of the monitor scenario', () => {
  it('matches the Node store strictly, frame by frame', async () => {
    const ops: Record<string, unknown>[] = []
    const node = await monitorScenario({ ops })
    const doc = replayLib.opsDocument(node, ops)
    const { trace } = replayLib.replay(doc, bin)
    const out = process.env.CCM_TRACE_OUT
    if (out) {
      await mkdir(out, { recursive: true })
      await writeFile(path.join(out, 'monitor.ops.json'), JSON.stringify(doc))
      await writeFile(path.join(out, 'monitor-node.trace.json'), JSON.stringify(node))
      await writeFile(path.join(out, 'monitor-rust.trace.json'), JSON.stringify(trace))
    }
    expect(compare.compareTraces(node, trace, { strict: true }).diffs).toEqual([])
    expect(compare.compareTraces(node, trace).diffs).toEqual([])
    const frames = (t: Trace) => t.steps.map((s) => s.frames)
    expect(JSON.parse(JSON.stringify(frames(trace)))).toEqual(frames(node))
  })
})
