import { appendFile, mkdir, mkdtemp, rm, truncate, writeFile } from 'node:fs/promises'
import os from 'node:os'
import path from 'node:path'
import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it } from 'vitest'
import { onCoreFallback, resetCoreFallback, type CoreFallbackMessage } from '../src/coreFallback'
import { createCoreTranscript, type CoreTranscriptClient } from '../src/coreTranscript'
import { CoreRollouts, CoreRolloutSource } from '../src/codex/coreRollout'
import type { ClaudeTarget } from '../src/codex/link'
import { CodexMonitor } from '../src/codex/monitor'
import { NodeRolloutSource, nodeRollouts } from '../src/codex/nodeRollout'
import type { CodexProcess } from '../src/codex/processes'
import { codexCoreMode, createRolloutSources, type RolloutSources } from '../src/codex/rolloutSource'
import { MonitorStore } from '../src/store'
import { call, CHILD, execCall, patchCall, complete, context, corpus, line, message, meta, output, started, T0, TID, tokens } from './codex-parity-corpus'
import { BIN } from './tracker-twin'

const describeCore = BIN ? describe : describe.skip
const plain = (v: unknown) => JSON.parse(JSON.stringify(v)) as unknown
const tally = { reads: 0, records: 0, scenarios: 0 }
const dirs: string[] = []
const temp = async (p: string) => {
  const d = await mkdtemp(path.join(os.tmpdir(), p))
  dirs.push(d)
  return d
}

let client: CoreTranscriptClient | null = null
const rust = (opts: { maxBytes?: number; chunkBytes?: number } = {}) => new CoreRollouts({ client: () => client, ...opts })

afterEach(() => {
  onCoreFallback(null)
  resetCoreFallback()
})

describe('codex rollout source', () => {
  it('reads in Rust unless CCM_CORE_CODEX=node or no core binary is set', () => {
    expect(codexCoreMode({})).toBe('rust')
    expect(codexCoreMode({ CCM_CORE_CODEX: 'node' })).toBe('node')
    expect(codexCoreMode({ CCM_CORE_CODEX: ' Rust ' })).toBe('rust')
    const got: CoreFallbackMessage[] = []
    onCoreFallback((m) => got.push(m))
    expect(createRolloutSources({ CCM_CORE_CODEX: 'node', CCM_CORE_BIN: 'x' })).toBe(nodeRollouts)
    expect(createRolloutSources({})).toBe(nodeRollouts)
    expect(createRolloutSources({ CCM_CORE_BIN: 'x' })).toBeInstanceOf(CoreRollouts)
    expect(createRolloutSources({ CCM_CORE_CODEX: 'rust', CCM_CORE_BIN: 'x' })).toBeInstanceOf(CoreRollouts)
    expect(got.map((m) => m.kind)).toEqual(['codex'])
  })

  it('falls back to Node with one notification when the core gives no answer', async () => {
    const dir = await temp('ccm-codex-fb-')
    const file = path.join(dir, `rollout-x-${TID}.jsonl`)
    await writeFile(file, line(meta()) + line(started()) + line(tokens()))
    const got: CoreFallbackMessage[] = []
    onCoreFallback((m) => got.push(m))
    const dead: CoreTranscriptClient = { request: async () => null, send: () => false, dispose: () => undefined }
    const core = new CoreRollouts({ client: () => dead, log: () => undefined })
    expect(await core.meta(file)).toEqual(await nodeRollouts.meta(file))
    const source = core.tail(file) as CoreRolloutSource
    expect(plain(await source.read())).toEqual(plain(await new NodeRolloutSource(file).read()))
    expect(source.usesNode).toBe(true)
    expect(plain(await core.once(file))).toEqual(plain(await nodeRollouts.once(file)))
    expect(got.map((m) => m.kind)).toEqual(['codex'])
    expect(core.fellBack).toBe(true)
  })
})

describeCore('codex rollouts: Node ↔ Rust parity', () => {
  beforeAll(() => {
    client = createCoreTranscript({ bin: BIN!, timeoutMs: 60_000 })
  })

  afterAll(async () => {
    client?.dispose()
    console.log(`[codex parity] ${tally.reads} reads, ${tally.records} records, ${tally.scenarios} monitor scenarios compared`)
    for (const d of dirs) await rm(d, { recursive: true, force: true })
  })

  const twin = (file: string, fromEnd?: number, chunk?: number, maxBytes?: number) => {
    const ts = new NodeRolloutSource(file, fromEnd, chunk)
    const rs = rust({ chunkBytes: chunk, maxBytes }).tail(file, fromEnd)
    return async () => {
      const a = plain(await ts.read())
      const b = await rs.read()
      expect(b).toEqual(a)
      expect((rs as CoreRolloutSource).usesNode).toBe(false)
      tally.reads++
      tally.records += b.length
      return b
    }
  }

  it('reads the meta line like Node, with or without its newline, and none for a non-meta first line', async () => {
    const dir = await temp('ccm-codex-meta-')
    const file = path.join(dir, `rollout-z-${TID}.jsonl`)
    const core = rust()
    const variants = [
      line(meta()),
      JSON.stringify(meta()),
      JSON.stringify(meta({ base_instructions: 'z'.repeat(5_000_000) })),
      JSON.stringify(meta({ cwd: 'C:\\dự án\\"q"\\x', source: 'cli' })),
      line(started()),
      'garbage\n',
      '',
    ]
    for (const v of variants) {
      await writeFile(file, v)
      expect(await core.meta(file)).toEqual(plain(await nodeRollouts.meta(file)))
    }
    expect(await core.meta(path.join(dir, 'missing.jsonl'))).toBeNull()
    expect(core.fellBack).toBe(false)
  })

  it('tails partial lines, big skipped lines and a compaction written in pieces like Node', async () => {
    const dir = await temp('ccm-codex-tail-')
    const file = path.join(dir, `rollout-x-${TID}.jsonl`)
    await writeFile(file, line(meta()) + line(started()) + line(message()))
    const read = twin(file)
    await read()
    const big = JSON.stringify({ type: 'compacted', timestamp: '2026-10-11T07:00:00.000Z', payload: { message: 'z'.repeat(300_000) } })
    await appendFile(file, big.slice(0, 1000))
    await read()
    await appendFile(file, `${big.slice(1000)}\n${JSON.stringify(tokens()).slice(0, 40)}`)
    await read()
    await appendFile(file, `${JSON.stringify(tokens()).slice(40)}\n`)
    const last = await read()
    expect(JSON.stringify(last)).not.toContain('secret')
  })

  it('matches Node on a large generated rollout read in random pieces, cut, truncated and resumed', async () => {
    const dir = await temp('ccm-codex-big-')
    const file = path.join(dir, `rollout-2026-10-11T14-00-00-${TID}.jsonl`)
    for (const seed of [1, 7, 42]) {
      const text = Buffer.from(corpus(seed, 400))
      await writeFile(file, '')
      const small = twin(file, undefined, 4096, 7000)
      const whole = twin(file)
      let at = 0
      let step = 0
      while (at < text.length) {
        const n = Math.max(1, Math.floor(((seed * 7919 + step * 104729) % 1000) / 1000 * 300_000))
        await appendFile(file, text.subarray(at, at + n))
        at += n
        step++
        await small()
        if (step % 3 === 0) await whole()
      }
      await whole()
      expect(tally.records).toBeGreaterThan(0)
      const resumed = twin(file, 1_000_000 + seed * 333, 4096, 5000)
      await resumed()
      await truncate(file, Math.floor(text.length / 3))
      await appendFile(file, text.subarray(0, 200_000))
      await small()
      await whole()
      await resumed()
      await appendFile(file, `\n${line(started())}${line(call('r1'))}${line(tokens())}`)
      await small()
      await whole()
      const tail = await resumed()
      expect(tail.flat().map((s) => s.k)).toEqual(['seen', 'turn_start', 'seen', 'tool_call', 'seen', 'tokens', 'limits'])
    }
  }, 120_000)

  it('carries on in Node from where the core stopped, without losing or repeating a record', async () => {
    const dir = await temp('ccm-codex-mid-')
    const file = path.join(dir, `rollout-x-${TID}.jsonl`)
    const text = Buffer.from(corpus(5, 300))
    await writeFile(file, text.subarray(0, text.length >> 1))
    let left = 4
    const flaky: CoreTranscriptClient = { request: (op, extra, o) => (left-- > 0 ? client!.request(op, extra, o) : Promise.resolve(null)), send: (op, extra) => client!.send(op, extra), dispose: () => undefined }
    const core = new CoreRollouts({ client: () => flaky, maxBytes: 3000, chunkBytes: 4096, log: () => undefined })
    const rs = core.tail(file) as CoreRolloutSource
    const ts = new NodeRolloutSource(file, undefined, 4096)
    const a = [...(await ts.read())]
    const b = [...(await rs.read())]
    expect(rs.usesNode).toBe(true)
    await appendFile(file, text.subarray(text.length >> 1))
    a.push(...(await ts.read()))
    b.push(...(await rs.read()))
    expect(b).toEqual(plain(a))
    expect(core.fellBack).toBe(true)
  })

  describe('CodexMonitor', () => {
    let home: string
    let dayDir: string
    let now: number
    let procs: CodexProcess[] | null
    let alive: Set<number>
    let targets: ClaudeTarget[]

    beforeEach(async () => {
      home = await temp('ccm-codex-home-')
      now = T0 + 10_000
      const d = new Date(now)
      const pad = (n: number) => String(n).padStart(2, '0')
      dayDir = path.join(home, 'sessions', String(d.getFullYear()), pad(d.getMonth() + 1), pad(d.getDate()))
      await mkdir(dayDir, { recursive: true })
      procs = []
      alive = new Set()
      targets = []
    })

    const fileOf = (id: string) => path.join(dayDir, `rollout-2026-10-11T14-00-00-${id}.jsonl`)
    const write = (id: string, lines: unknown[]) => writeFile(fileOf(id), lines.map(line).join(''))

    type Step = (codex: CodexMonitor, store: MonitorStore) => Promise<void>
    const tick: Step = (codex) => (codex as unknown as { tick(): Promise<void> }).tick()
    const sync = (id: string): Step => (codex) => (codex as unknown as { sync(id: string, file: string): Promise<void> }).sync(id, fileOf(id))
    const append = (id: string, lines: unknown[]): Step => () => appendFile(fileOf(id), lines.map(line).join(''))

    const view = (store: MonitorStore) => {
      const s = store.snapshot()
      return plain({ terminals: s.terminals, agents: s.agents, codexLimits: s.codexLimits, activity: s.activity.map(({ id: _id, at: _at, ...rest }) => rest) })
    }

    const run = async (setup: () => Promise<void>, steps: Step[], init: (s: MonitorStore) => void = () => undefined) => {
      const views: unknown[][] = [[], []]
      for (const [i, rollouts] of [nodeRollouts, rust()].entries() as Iterable<[number, RolloutSources]>) {
        await rm(path.join(home, 'sessions'), { recursive: true, force: true })
        await mkdir(dayDir, { recursive: true })
        await setup()
        const store = new MonitorStore({ activityLimit: 100, batchMs: 1 })
        procs = []
        alive = new Set()
        targets = []
        init(store)
        const codex = new CodexMonitor({ codexHome: home, staleTtlMs: 120 * 60_000, reconcileMs: 5_000, verifyMs: 60_000, verifyProcesses: true }, store, () => targets, { listProcesses: async () => procs, isPidAlive: (pid) => alive.has(pid), now: () => now }, null, rollouts)
        await codex.start({ watch: false })
        views[i]!.push(view(store))
        for (const step of steps) {
          await step(codex, store)
          views[i]!.push(view(store))
        }
        codex.stop()
      }
      expect(views[1]).toEqual(views[0])
      tally.scenarios++
      return views[0]!
    }

    it('places a Claude-launched exec under the calling agent and cancels it with the process', async () => {
      const init = (s: MonitorStore) => {
        s.upsertTerminal({ id: 'claude-1', title: 'demo', status: 'working' })
        targets = [{ terminalId: 'claude-1', pid: 100, mainAgentId: 'claude-1:main', ended: false, codexCalls: [{ agentId: 'sub-a', at: new Date(T0 - 1_000).toISOString() }] }]
        procs = [{ pid: 300, start: T0 - 200, mode: 'exec', ancestors: [250, 100], orphan: false }]
        alive.add(300)
      }
      const kill: Step = async (codex, store) => {
        alive.delete(300)
        await tick(codex, store)
      }
      const views = await run(() => write(TID, [meta(), started(), context(), call('c1'), tokens()]), [append(TID, [output('c1'), call('c2')]), sync(TID), kill], init)
      expect(JSON.stringify(views)).not.toContain('secret')
      expect(JSON.stringify(views)).toContain('"parentId":"sub-a"')
    })

    it('publishes the running command summary, masked, and clears it with the call output', async () => {
      const init = (s: MonitorStore) => {
        s.upsertTerminal({ id: 'claude-1', title: 'demo', status: 'working' })
        targets = [{ terminalId: 'claude-1', pid: 100, mainAgentId: 'claude-1:main', ended: false, codexCalls: [{ agentId: 'claude-1:main', at: new Date(T0 - 1_000).toISOString() }] }]
        procs = [{ pid: 300, start: T0 - 200, mode: 'exec', ancestors: [250, 100], orphan: false }]
        alive.add(300)
      }
      const views = await run(
        () => write(TID, [meta(), started(), context(), execCall('c1', 'npm test -- --password hunter22', T0 + 10), tokens()]),
        [append(TID, [output('c1'), patchCall('c2', T0 + 20)]), sync(TID), append(TID, [output('c2')]), sync(TID)],
        init,
      )
      const agents = views.map((v) => (v as { agents: { provider?: string; currentToolSummary?: string; currentToolStartedAt?: string }[] }).agents.find((a) => a.provider === 'codex'))
      expect(agents.map((a) => [a?.currentToolSummary, a?.currentToolStartedAt])).toEqual([
        ['npm test -- --password [masked]', new Date(T0 + 10).toISOString()],
        ['npm test -- --password [masked]', new Date(T0 + 10).toISOString()],
        ['src/a.ts', new Date(T0 + 20).toISOString()],
        ['src/a.ts', new Date(T0 + 20).toISOString()],
        [undefined, undefined],
      ])
      expect(JSON.stringify(views)).not.toContain('hunter22')
      expect(JSON.stringify(views)).not.toContain('secret')
    })

    it('shows other runs on their own with their subagent, ends them with the process and reopens a resumed thread', async () => {
      const desktop = '01a125e6-0000-7000-8000-000000000001'
      const setup = async () => {
        await write(TID, [meta(), started(), context()])
        await write(desktop, [meta({ id: desktop, session_id: desktop, source: 'vscode', originator: 'Codex Desktop' }), started()])
        await write(CHILD, [meta({ id: CHILD, session_id: CHILD, source: { subagent: { thread_spawn: { parent_thread_id: TID, depth: 1 } } } }), started(), call('k1')])
      }
      const init = () => {
        procs = [{ pid: 400, start: T0 - 100, mode: 'exec', ancestors: [1], orphan: false }]
        alive.add(400)
      }
      const exit: Step = async (codex, store) => {
        alive.delete(400)
        await tick(codex, store)
      }
      const views = await run(setup, [append(TID, [complete()]), sync(TID), append(CHILD, [output('k1'), complete()]), sync(CHILD), exit, append(TID, [started(T0 + 5_000), call('c9', T0 + 5_001)]), sync(TID)], init)
      expect(JSON.stringify(views[0])).toContain(`codex:${TID}`)
    })

    it('ignores finished and orphaned runs at startup but still reads the latest limits', async () => {
      const killed = '01a125e6-0000-7000-8000-000000000002'
      const views = await run(async () => {
        await write(TID, [meta(), started(), tokens(T0, 33), complete()])
        await write(killed, [meta({ id: killed, session_id: killed }), started(), call('c1')])
      }, [])
      expect(views[0]).toMatchObject({ terminals: [], codexLimits: { fiveHour: { usedPct: 33 } } })
    })
  })
})
