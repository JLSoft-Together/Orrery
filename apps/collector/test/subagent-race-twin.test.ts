import { appendFile, mkdir, mkdtemp, realpath, rm, writeFile } from 'node:fs/promises'
import os from 'node:os'
import path from 'node:path'
import { pathToFileURL } from 'node:url'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import { cwdSlug } from '../src/tracker'
import { MonitorTwin, SCRIPT_BIN } from './monitor-twin'
import type { Trace } from './trace-scenario'

process.env.CCM_CORE_TRANSCRIPT = 'node'
process.env.CCM_CORE_SIDEFILES = 'node'

interface CompareLib {
  compareTraces: (a: unknown, b: unknown, options?: { strict?: boolean; limit?: number }) => { diffs: string[]; steps: number; frames: number[] }
}

const SID = '0a1b2c3d-4e5f-4a6b-8c7d-9e0f1a2b3c4d'
const PID = 5151
const START = '134358400013055555'
const T0 = Date.parse('2026-10-07T10:00:00.000Z')
const END_AT = '2026-10-07T10:00:01.000Z'
const NOTE_AT = '2026-10-07T10:00:01.002Z'
const line = (o: unknown) => `${JSON.stringify(o)}\n`

type Order = 'sub-then-main' | 'together'

async function scenario(twin: MonitorTwin, root: string, order: Order): Promise<void> {
  const cwd = 'D:\\work\\race'
  const projectDir = path.join(root, 'projects', cwdSlug(cwd))
  const subDir = path.join(projectDir, SID, 'subagents')
  const main = path.join(projectDir, `${SID}.jsonl`)
  const sub = path.join(subDir, 'agent-r1.jsonl')
  await mkdir(path.join(root, 'sessions'), { recursive: true })
  await mkdir(subDir, { recursive: true })
  await writeFile(path.join(root, 'sessions', `${PID}.json`), JSON.stringify({ pid: PID, sessionId: SID, cwd, startedAt: 1_791_366_405_292, procStart: START, version: '2.1.289', kind: 'interactive', status: 'busy' }))
  await writeFile(
    main,
    line({ type: 'assistant', timestamp: '2026-10-07T09:59:00.000Z', message: { id: 'm1', model: 'claude-opus-5-5', stop_reason: 'tool_use', usage: { input_tokens: 100, output_tokens: 20 }, content: [{ type: 'tool_use', id: 'toolu_task', name: 'Agent', input: { prompt: 'p' } }] } }) +
      line({ type: 'user', timestamp: '2026-10-07T09:59:01.000Z', toolUseResult: { status: 'async_launched' }, message: { content: [{ type: 'tool_result', tool_use_id: 'toolu_task' }] } }),
  )
  await writeFile(path.join(subDir, 'agent-r1.meta.json'), JSON.stringify({ agentType: 'Explore', description: 'Race', toolUseId: 'toolu_task', spawnDepth: 1 }))
  await writeFile(
    sub,
    line({ type: 'assistant', isSidechain: true, timestamp: '2026-10-07T09:59:02.000Z', message: { id: 's1', stop_reason: 'tool_use', usage: { input_tokens: 7, output_tokens: 2 }, content: [{ type: 'tool_use', id: 'toolu_grep', name: 'Grep', input: {} }] } }) +
      line({ type: 'user', isSidechain: true, timestamp: '2026-10-07T09:59:03.000Z', message: { content: [{ type: 'tool_result', tool_use_id: 'toolu_grep', content: 'x' }] } }),
  )
  const n = twin.node
  const writeEnd = () => appendFile(sub, line({ type: 'assistant', isSidechain: true, timestamp: END_AT, message: { id: 's2', stop_reason: 'end_turn', usage: { input_tokens: 100, output_tokens: 300 }, content: [{ type: 'text', text: 'done' }] } }))
  const writeNote = () => appendFile(main, line({ type: 'queue-operation', operation: 'enqueue', timestamp: NOTE_AT, content: '<task-notification><task-id>r1</task-id><tool-use-id>toolu_task</tool-use-id><status>failed</status><summary>demo</summary></task-notification>' }))

  await twin.step('boot', async () => {
    await n.start({ watch: false })
    await twin.rustCmd({ cmd: 'start' })
  })
  if (order === 'sub-then-main') {
    await twin.step('sub-ends', async () => {
      await writeEnd()
      await twin.both(() => n.reconcile(), { cmd: 'reconcile' })
    })
    await twin.step('main-notified', async () => {
      await writeNote()
      await twin.both(() => n.reconcile(), { cmd: 'reconcile' })
    })
  } else {
    await twin.step('both-written', async () => {
      await writeEnd()
      await writeNote()
      await twin.both(() => n.reconcile(), { cmd: 'reconcile' })
      await twin.both(() => n.reconcile(), { cmd: 'reconcile' })
    })
  }
  await twin.step('final', async () => {
    await twin.snapshot()
  })
}

function outcome(trace: Trace): { status: unknown; completedAt: unknown; kinds: string[] } {
  const frames = trace.steps.flatMap((s) => s.frames)
  const snap = frames.flatMap((f) => f.events).filter((e) => e.type === 'snapshot').at(-1) as { payload: { agents: { id: string; status: unknown; completedAt?: unknown }[] } }
  const agent = snap.payload.agents.find((a) => a.id === 'r1')
  const kinds = frames.flatMap((f) => f.events).filter((e) => e.type === 'activity' && (e.payload as { agentId?: string }).agentId === 'r1').map((e) => (e.payload as { kind: string }).kind)
  return { status: agent?.status, completedAt: agent?.completedAt, kinds }
}

let compare: CompareLib

beforeAll(async () => {
  compare = (await import(pathToFileURL(path.resolve(__dirname, '../../../scripts/trace-compare.mjs')).href)) as CompareLib
})

describe.skipIf(!SCRIPT_BIN)('subagent outcome depends on when the main transcript is read (Node and Rust alike)', () => {
  const roots: string[] = []
  afterAll(async () => {
    for (const r of roots) await rm(r, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 })
  })

  const cases: [Order, string, string, string][] = [
    ['sub-then-main', 'completed', END_AT, 'agent.completed'],
    ['together', 'error', NOTE_AT, 'agent.failed'],
  ]
  for (const [order, status, at, kind] of cases) {
    it(`${order}: both sides say ${status} at ${at}`, async () => {
      const root = await realpath(await mkdtemp(path.join(os.tmpdir(), 'ccm-race-')))
      roots.push(root)
      const twin = new MonitorTwin(SCRIPT_BIN!, { root, now: T0 })
      try {
        await twin.init()
        await scenario(twin, root, order)
      } finally {
        twin.close()
      }
      const node = twin.trace('node')
      const rust = twin.trace('rust')
      expect(compare.compareTraces(node, rust).diffs).toEqual([])
      expect(compare.compareTraces(node, rust, { strict: true }).diffs).toEqual([])
      for (const side of [node, rust]) {
        const o = outcome(side)
        expect(o.status).toBe(status)
        expect(o.completedAt).toBe(at)
        expect(o.kinds).toContain(kind)
      }
    }, 60_000)
  }
})
