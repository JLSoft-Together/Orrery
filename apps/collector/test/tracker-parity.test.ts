import { appendFileSync } from 'node:fs'
import { appendFile, mkdir, mkdtemp, rm, utimes, writeFile } from 'node:fs/promises'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vitest'
import { createCoreTranscript, type CoreTranscriptClient } from '../src/coreTranscript'
import { CoreTracker, type CoreHub } from '../src/coreTracker'
import { SessionTracker, cwdSlug, terminalIdOf, type ContextHints } from '../src/tracker'
import { createCoreHub, createTracker, trackerMode } from '../src/trackerSource'
import { BIN, Twin, tally } from './tracker-twin'
import { assistant, CWD, line, msg, notification, NOW, realFixture, record, result, SID, SID2, system, toolResult, toolUse, use, type RealFixture } from './tracker-parity-corpus'

let client: CoreTranscriptClient | null = null
let hub: CoreHub
const dirs: string[] = []
const temp = async (p: string) => {
  const d = await mkdtemp(path.join(os.tmpdir(), p))
  dirs.push(d)
  return d
}

const describeCore = BIN ? describe : describe.skip

describe('tracker source', () => {
  it('picks the Rust tracker by default when a core binary is set, Node with CCM_CORE_TRANSCRIPT=node', () => {
    expect(trackerMode({})).toBe('rust')
    expect(trackerMode({ CCM_CORE_TRANSCRIPT: ' Node ' })).toBe('node')
    const make = (env: NodeJS.ProcessEnv) => createTracker('1:0', record(), 'p', {}, Date.now, env)
    expect(make({ CCM_CORE_TRANSCRIPT: 'rust' })).toBeInstanceOf(SessionTracker)
    expect(make({ CCM_CORE_BIN: 'x' })).toBeInstanceOf(CoreTracker)
    expect(make({ CCM_CORE_TRANSCRIPT: 'node', CCM_CORE_BIN: 'x' })).toBeInstanceOf(SessionTracker)
    expect(make({ CCM_CORE_TRANSCRIPT: 'rust', CCM_CORE_BIN: 'x' })).toBeInstanceOf(CoreTracker)
  })
})

describeCore('SessionTracker: Node ↔ Rust parity', () => {
  beforeAll(() => {
    client = createCoreTranscript({ bin: BIN!, timeoutMs: 60_000 })
    hub = createCoreHub(() => client)
  })

  afterAll(async () => {
    client?.dispose()
    console.log(`[tracker parity] ${tally.twins} trackers, ${tally.syncs} syncs, ${tally.activity} activity, ${tally.agents} agents, ${tally.spans} spans compared`)
    for (const d of dirs) await rm(d, { recursive: true, force: true })
  })

  describe('main ↔ subagent message counts', () => {
    const { AID, CHILD, at } = msg
    let projects: string
    let transcript: string
    let subDir: string
    let tracker: Twin
    const counts = (id: string) => {
      const a = tracker.buildAgents(tracker.buildTerminal()).find((x) => x.id === id)
      return a && { msgIn: a.msgIn, msgOut: a.msgOut }
    }
    const addSubagent = async (id: string, body: string, meta: Record<string, unknown> = {}) => {
      await writeFile(path.join(subDir, `agent-${id}.meta.json`), JSON.stringify({ agentType: 'Explore', toolUseId: `toolu_${id}`, ...meta }))
      await writeFile(path.join(subDir, `agent-${id}.jsonl`), body)
    }

    beforeEach(async () => {
      projects = await temp('ccm-t3-msg-')
      transcript = path.join(projects, cwdSlug(CWD), `${SID}.jsonl`)
      subDir = path.join(projects, cwdSlug(CWD), SID, 'subagents')
      await mkdir(subDir, { recursive: true })
      await writeFile(transcript, toolUse('m1', `toolu_${AID}`, 'Agent', at(0)))
      tracker = new Twin(terminalIdOf(record()), record(), projects, {}, () => Date.parse(at(60)), hub)
    })

    it('builds counts at catch-up and adds live messages', async () => {
      await appendFile(transcript, msg.enqueue(AID, at(5)) + msg.handbackUser(AID, 'u-h1', at(5)) + msg.queuedPeer(msg.peer(AID, false), 'src-p1', at(8)) + msg.queuedPeer(msg.peer(AID), 'src-h2', at(9)))
      await addSubagent(AID, msg.coordinator('u-c1', at(1)) + msg.coordinatorQueued('src-c2', at(2)) + msg.sendMain('toolu_sm1', at(3)))
      expect(await tracker.sync(true)).toEqual([])
      expect(counts(AID)).toEqual({ msgIn: 2, msgOut: 3 })
      await appendFile(path.join(subDir, `agent-${AID}.jsonl`), msg.coordinator('u-c1', at(1)) + msg.coordinator('u-c2', at(10)) + msg.sendMain('toolu_sm2', at(12)))
      expect((await tracker.sync(false)).map((a) => a.kind)).toEqual(['tool.started'])
      tracker.drainPeers()
    })

    it('keeps a hand-back for an agent it has not seen yet and credits nested hand-backs', async () => {
      await appendFile(transcript, msg.handbackUser(AID, 'u-h1', at(5)))
      await tracker.sync(true)
      await addSubagent(AID, toolUse('s1', `toolu_${CHILD}`, 'Agent', at(1)) + msg.handbackUser(CHILD, 'u-h2', at(4)))
      await addSubagent(CHILD, '', { parentAgentId: AID, spawnDepth: 2 })
      expect((await tracker.sync(false)).map((a) => a.kind)).toEqual(['agent.started', 'agent.started', 'tool.started'])
      expect(counts(CHILD)).toEqual({ msgIn: undefined, msgOut: 1 })
    })

    it('forgets counts when the registry switches session', async () => {
      await addSubagent(AID, msg.coordinator('u-c1', at(1)))
      await tracker.sync(true)
      tracker.setRecord(record({ sessionId: SID2 }))
      await writeFile(path.join(projects, cwdSlug(CWD), `${SID2}.jsonl`), '')
      const dir2 = path.join(projects, cwdSlug(CWD), SID2, 'subagents')
      await mkdir(dir2, { recursive: true })
      await writeFile(path.join(dir2, `agent-${AID}.meta.json`), JSON.stringify({ agentType: 'Explore' }))
      await writeFile(path.join(dir2, `agent-${AID}.jsonl`), '')
      await tracker.sync(true)
      expect(counts(AID)).toEqual({ msgIn: undefined, msgOut: undefined })
    })
  })

  describe('live tracking', () => {
    let projects: string
    let transcript: string
    let tracker: Twin
    const dir = () => path.join(projects, cwdSlug(CWD), SID, 'subagents')
    const hints: ContextHints = {
      sessionWindows: new Map(),
      sessionContext: new Map(),
      sensitive: { check: (_t, touch, cwd) => [{ rule: 'env', path: JSON.stringify(touch).slice(0, 40), file: cwd, project: 'demo' } as never] },
    }

    beforeEach(async () => {
      projects = await temp('ccm-t3-live-')
      await mkdir(path.join(projects, cwdSlug(CWD)))
      transcript = path.join(projects, cwdSlug(CWD), `${SID}.jsonl`)
      await writeFile(transcript, toolUse('m1', 't1', 'Read', '2026-10-07T10:00:00.000Z') + toolResult('t1', '2026-10-07T10:00:01.000Z'))
      tracker = new Twin(terminalIdOf(record()), record(), projects, hints, () => Date.parse('2026-10-07T10:30:00.000Z'), hub)
    })

    it('reports tool calls, failures, sensitive touches, compaction and mode changes', async () => {
      expect(await tracker.sync(true)).toEqual([])
      await appendFile(
        transcript,
        toolUse('m2', 't2', 'Bash', '2026-10-07T10:00:02.000Z', { command: 'cat .env && codex exec x' }) +
          toolResult('t2', '2026-10-07T10:00:03.000Z', true) +
          toolUse('m3', 't3', 'Edit', '2026-10-07T10:00:04.000Z', { file_path: 'D:\\work\\demo\\.env' }) +
          line({ type: 'permission-mode', permissionMode: 'plan', sessionId: SID }) +
          line({ type: 'permission-mode', permissionMode: 'acceptEdits', sessionId: SID }) +
          system('compact_boundary', '2026-10-07T10:01:00.000Z', { compactMetadata: { trigger: 'manual', preTokens: 1000, postTokens: 100 } }),
      )
      const activity = await tracker.sync(false)
      expect(activity.map((a) => a.kind)).toEqual(['tool.started', 'tool.failed', 'tool.started', 'sensitive', 'terminal.mode', 'terminal.compacted'])
      expect(tracker.drainSpans().map((s) => s.id)).toEqual(['t2', 't2', 't3'])
      expect(tracker.ts.codexCalls()).toHaveLength(1)
    })

    it('shows status, context hints, end and session switch', async () => {
      hints.sessionWindows!.set(SID, 200_000)
      hints.sessionContext!.set(SID, { pct: 12, bufferPct: 10, at: '2026-10-07T11:00:00.000Z' })
      tracker.setRecord(record({ status: 'waiting', waitingFor: 'permission', statusUpdatedAt: 1_791_366_500_000, name: 'demo-x' }))
      await appendFile(transcript, toolUse('m2', 't2', 'Bash', '2026-10-07T10:00:02.000Z'))
      await tracker.sync(true)
      tracker.setRecord(record({ status: 'idle' }))
      expect(tracker.closeOpenSpans('2026-10-07T10:05:00.000Z')).toHaveLength(1)
      tracker.markEnded('2026-10-07T11:00:00.000Z')
      await writeFile(path.join(projects, cwdSlug(CWD), `${SID2}.jsonl`), toolUse('n1', 'u1', 'Edit', '2026-10-07T10:05:00.000Z'))
      expect(tracker.setRecord(record({ sessionId: SID2 }))).toEqual({ sessionChanged: true })
      await tracker.sync(true)
      hints.sessionWindows!.clear()
      hints.sessionContext!.clear()
    })

    it('coalesces overlapping syncs without losing appended records', async () => {
      await tracker.sync(true)
      appendFileSync(transcript, toolUse('m2', 't2', 'Bash', '2026-10-07T10:00:02.000Z'))
      const first = tracker.sync(false)
      appendFileSync(transcript, toolUse('m3', 't3', 'Grep', '2026-10-07T10:00:04.000Z'))
      const second = tracker.sync(false)
      const [a, b] = await Promise.all([first, second])
      expect(b).toEqual([])
      expect(a.filter((x) => x.kind === 'tool.started').map((x) => x.data?.tool)).toEqual(['Bash', 'Grep'])
    })

    it('tracks subagents: started, tools, foreground result closes spans, unknown after end', async () => {
      await mkdir(dir(), { recursive: true })
      await appendFile(transcript, toolUse('m2', 'toolu_fg', 'Agent', '2026-10-07T10:00:02.000Z'))
      await tracker.sync(true)
      await writeFile(path.join(dir(), 'agent-a1.meta.json'), JSON.stringify({ agentType: 'Explore', description: 'x'.repeat(300), toolUseId: 'toolu_agent', parentAgentId: '../evil' }))
      await writeFile(path.join(dir(), 'agent-a1.jsonl'), toolUse('s1', 'g1', 'Grep', '2026-10-07T10:00:05.000Z'))
      await writeFile(path.join(dir(), 'agent-f1.meta.json'), JSON.stringify({ agentType: 'Plan', toolUseId: 'toolu_fg', model: 'claude-x' }))
      await writeFile(path.join(dir(), 'agent-f1.jsonl'), toolUse('s2', 'g2', 'Glob', '2026-10-07T10:00:06.000Z'))
      await tracker.sync(false)
      tracker.drainSpans()
      await appendFile(transcript, toolResult('toolu_fg', '2026-10-07T10:00:09.000Z'))
      expect((await tracker.sync(false)).map((a) => a.kind)).toEqual(['agent.completed'])
      expect(tracker.drainSpans()).toContainEqual(expect.objectContaining({ id: 'g2', agentId: 'f1', outcome: 'interrupted' }))
      tracker.markEnded('2026-10-07T11:00:00.000Z')
      await appendFile(path.join(dir(), 'agent-a1.jsonl'), toolResult('g1', '2026-10-07T10:00:10.000Z'))
      await tracker.sync(false, { files: [path.join(dir(), 'agent-a1.jsonl')] })
    })

    it('carries reply text, ask, limit hit and device / port signals', async () => {
      await tracker.sync(true)
      const ask = { questions: [{ question: 'Ship it?', options: [{ label: 'Yes' }, { label: 'No' }], multiSelect: false }] }
      await appendFile(
        transcript,
        assistant('r1', '2026-10-07T10:02:00.000Z', [{ type: 'text', text: 'Hello there' }], null) +
          assistant('r1', '2026-10-07T10:02:01.000Z', [{ type: 'text', text: 'second part' }], 'end_turn') +
          assistant('a1', '2026-10-07T10:03:00.000Z', [use('toolu_ask', 'AskUserQuestion', ask), use('toolu_adb', 'Bash', { command: 'adb devices' })], 'tool_use') +
          result('2026-10-07T10:03:01.000Z', 'toolu_adb', { error: true }).replace('"x', '"Error: listen EADDRINUSE: address already in use :::3000 x') +
          line({ type: 'assistant', timestamp: '2026-10-07T10:04:00.000Z', isApiErrorMessage: true, error: 'rate_limit', message: { id: 'lim', model: '<synthetic>', content: [{ type: 'text', text: "You've hit your limit · resets 3pm" }] } }),
      )
      await tracker.sync(false)
      tracker.drainPeers()
    })
  })

  describe('catch-up on real-shaped transcripts', () => {
    let fx: RealFixture
    const twin = () => new Twin(terminalIdOf(record()), record(), fx.projects, {}, () => NOW, hub)

    beforeEach(async () => {
      fx = await realFixture()
      dirs.push(fx.projects)
    })

    it('builds the same terminal, agents and spans after catch-up and live lines', async () => {
      const tracker = twin()
      expect(await tracker.sync(true)).toEqual([])
      await appendFile(fx.transcript, result('2026-10-07T13:41:00.000Z', 'toolu_r1') + system('turn_duration', '2026-10-07T13:41:01.000Z') + `{"type":"attachment","gitBranch":"feature/y"}\n`)
      await appendFile(path.join(fx.subDir, 'agent-live.jsonl'), result('2026-10-07T13:41:03.000Z', 'toolu_l1', { error: true }) + assistant('l2', '2026-10-07T13:41:04.000Z', [{ type: 'text', text: 'ok' }], 'end_turn'))
      await appendFile(fx.transcript, notification('2026-10-07T13:41:05.000Z', 'toolu_live', 'completed'))
      expect((await tracker.sync(false)).length).toBeGreaterThan(0)
      tracker.drainSpans()
      tracker.toolSpans(Date.parse('2026-10-07T13:00:00.000Z'))
    })

    it('keeps live spans past the span cap and counts messages in a skipped head', async () => {
      const bg = path.join(fx.subDir, 'agent-bg.jsonl')
      let more = msg.coordinator('u-c1', '2026-10-07T10:00:02.500Z')
      for (let i = 0; i < 2100; i++) more += assistant(`x${i}`, '2026-10-07T10:02:00.000Z', [use(`toolu_x${i}`, 'Read')], 'tool_use') + result('2026-10-07T10:02:01.000Z', `toolu_x${i}`)
      const { readFile } = await import('node:fs/promises')
      await writeFile(bg, more + (await readFile(bg, 'utf8')))
      await utimes(bg, fx.old, fx.old)
      const tracker = twin()
      await tracker.sync(true)
      expect(tracker.toolSpans().map((s) => s.id)).toEqual(['toolu_live', 'toolu_b1', 'toolu_r1', 'toolu_l1'])
    })

    it('spans over the cap on the main transcript drop the same ids', async () => {
      let lines = ''
      for (let i = 0; i < 2300; i++) lines += assistant(`y${i}`, '2026-10-07T13:00:00.000Z', [use(`toolu_y${i}`, 'Read')], 'tool_use') + (i % 7 ? result('2026-10-07T13:00:01.000Z', `toolu_y${i}`) : '')
      await appendFile(fx.transcript, lines)
      const tracker = twin()
      await tracker.sync(true)
      await appendFile(fx.transcript, lines.replaceAll('toolu_y', 'toolu_z'))
      await tracker.sync(false)
      expect(tracker.drainSpans().length).toBeGreaterThan(2000)
    })

    it('reads only the files a targeted sync names and leaves finished subagents on skip-resolved', async () => {
      const tracker = twin()
      await tracker.sync(true)
      const live = path.join(fx.subDir, 'agent-live.jsonl')
      await appendFile(fx.transcript, assistant('m6', '2026-10-07T13:42:00.000Z', [use('toolu_x', 'Edit')], 'tool_use'))
      await appendFile(live, assistant('l2', '2026-10-07T13:42:01.000Z', [use('toolu_l2', 'Glob')], 'tool_use'))
      expect((await tracker.sync(false, { files: [live] })).map((a) => a.agentId)).toEqual(['live'])
      expect((await tracker.sync(false, { files: [fx.transcript] })).map((a) => a.agentId)).toEqual([tracker.mainAgentId])
      expect(await tracker.sync(false, { files: [path.join(fx.subDir, 'unrelated.txt')] })).toEqual([])
      await appendFile(path.join(fx.subDir, 'agent-bg.jsonl'), assistant('late', '2026-10-07T13:50:00.000Z', [{ type: 'text', text: 'more' }], 'end_turn'))
      await tracker.sync(false, { skipResolved: true })
      await tracker.sync(false)
    })
  })

  describe('core failure', () => {
    let fx: RealFixture

    beforeEach(async () => {
      fx = await realFixture()
      dirs.push(fx.projects)
    })

    it('reopens in the core after a lost reply and keeps the same state', async () => {
      let fail = 0
      const flaky: CoreTranscriptClient = {
        request: (op, extra, o) => (op === 'tracker.sync' && fail-- > 0 ? Promise.resolve(null) : client!.request(op, extra, o)),
        send: (op, extra) => client!.send(op, extra),
        dispose: () => undefined,
      }
      const ts = new SessionTracker('9:1', record(), fx.projects, {}, () => NOW)
      const rust = new CoreTracker('9:1', record(), fx.projects, {}, () => NOW, createCoreHub(() => flaky))
      await Promise.all([ts.sync(true), rust.sync(true)])
      await appendFile(fx.transcript, assistant('m9', '2026-10-07T13:45:00.000Z', [use('toolu_q', 'Grep')], 'tool_use'))
      fail = 1
      await Promise.all([ts.sync(false), rust.sync(false)])
      expect(rust.usingNode).toBe(false)
      const t = ts.buildTerminal()
      expect(rust.buildTerminal()).toEqual(t)
      expect(rust.buildAgents(t)).toEqual(ts.buildAgents(t))
      expect(rust.toolSpans()).toEqual(ts.toolSpans())
      expect(rust.drainSpans().map((x) => x.id)).toEqual(ts.drainSpans().map((x) => x.id))
    })

    it('falls back to the Node tracker when the core cannot start', async () => {
      const logs: string[] = []
      const dead = createCoreHub(() => createCoreTranscript({ bin: path.join(fx.projects, 'missing-core.exe'), log: (l) => logs.push(l) }), (l) => logs.push(l))
      const ts = new SessionTracker('9:2', record(), fx.projects, {}, () => NOW)
      const rust = new CoreTracker('9:2', record(), fx.projects, {}, () => NOW, dead)
      rust.shell = 'pwsh'
      ts.shell = 'pwsh'
      await Promise.all([ts.sync(true), rust.sync(true)])
      expect(rust.usingNode).toBe(true)
      await appendFile(fx.transcript, assistant('m9', '2026-10-07T13:45:00.000Z', [use('toolu_q', 'Grep')], 'tool_use'))
      const [a, b] = await Promise.all([ts.sync(false), rust.sync(false)])
      expect(b).toEqual(a)
      const t = ts.buildTerminal()
      expect(rust.buildTerminal()).toEqual(t)
      expect(rust.buildAgents(t)).toEqual(ts.buildAgents(t))
      expect(logs.some((l) => l.includes('using the Node session tracker'))).toBe(true)
    })
  })
})
