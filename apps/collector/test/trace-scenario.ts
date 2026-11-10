import { appendFile, mkdir, mkdtemp, realpath, rm, writeFile } from 'node:fs/promises'
import os from 'node:os'
import path from 'node:path'
import type { MonitorEvent, ServerMessage } from '@ccm/shared'
import type { CollectorConfig } from '../src/config'
import { Monitor } from '../src/monitor'
import { MonitorStore } from '../src/store'

const SID = '11111111-2222-3333-4444-555555555555'
const SID2 = '66666666-7777-8888-9999-000000000000'
const PID = 4242
const CWD = 'D:\\work\\demo'
const SLUG = 'D--work-demo'

export interface TraceStep {
  name: string
  sent: unknown[]
  frames: ServerMessage[]
}

export interface Trace {
  format: 'ccm-trace/1'
  scenario: string
  source: 'node'
  wall: { start: number; end: number }
  pids: Record<string, string>
  replace: [string, string][]
  ignore: string[]
  steps: TraceStep[]
}

const line = (o: unknown) => `${JSON.stringify(o)}\n`

function registry(over: Record<string, unknown> = {}) {
  return JSON.stringify({
    pid: PID,
    sessionId: SID,
    cwd: CWD,
    startedAt: 1_791_366_405_292,
    procStart: '134358400013081891',
    version: '2.1.289',
    kind: 'interactive',
    entrypoint: 'cli',
    name: 'demo-1a',
    nameSource: 'derived',
    status: 'busy',
    statusUpdatedAt: 1_791_366_405_292,
    messagingSocketPath: '\\\\.\\pipe\\LOCAL\\cc-msg-secret',
    ...over,
  })
}

function config(root: string): CollectorConfig {
  return {
    claudeRoot: root,
    codexHome: root,
    codex: false,
    codewhaleHome: root,
    deepseek: false,
    host: '127.0.0.1',
    port: 0,
    toast: false,
    toastBackend: null,
    appKey: null,
    dev: true,
    toastLink: null,
    staleTtlMs: 120 * 60_000,
    reconcileMs: 5_000,
    processVerifyMs: 60_000,
    activityLimit: 300,
    batchMs: 1,
    verifyProcesses: false,
    webDist: root,
    dataDir: root,
    devOriginPorts: [],
    statusLineBridge: path.join(root, 'statusline-bridge.mjs'),
    statusLineCommand: 'node statusline-bridge.mjs',
  }
}

async function seed(root: string, projectDir: string): Promise<void> {
  await mkdir(path.join(root, 'sessions'), { recursive: true })
  await mkdir(path.join(projectDir, SID, 'subagents'), { recursive: true })
  await writeFile(path.join(root, 'sessions', `${PID}.json`), registry())
  await writeFile(
    path.join(projectDir, `${SID}.jsonl`),
    line({ type: 'assistant', timestamp: '2026-10-07T09:59:00.000Z', message: { id: 'm1', model: 'claude-opus-5-5', stop_reason: 'tool_use', usage: { input_tokens: 100, output_tokens: 20 }, content: [{ type: 'tool_use', id: 'toolu_agent', name: 'Agent', input: { prompt: 'p' } }] } }) +
      line({ type: 'user', timestamp: '2026-10-07T09:59:01.000Z', toolUseResult: { status: 'async_launched' }, message: { content: [{ type: 'tool_result', tool_use_id: 'toolu_agent' }] } }) +
      line({ type: 'assistant', timestamp: '2026-10-07T09:59:02.000Z', message: { id: 'm2', stop_reason: 'tool_use', usage: { input_tokens: 50, output_tokens: 5 }, content: [{ type: 'tool_use', id: 'toolu_bash', name: 'Bash', input: { command: 'echo' } }] } }),
  )
  await writeFile(
    path.join(projectDir, SID, 'subagents', 'agent-a1.meta.json'),
    JSON.stringify({ agentType: 'Explore', description: 'Find ads code', toolUseId: 'toolu_agent', spawnDepth: 1, requestShape: 'background' }),
  )
  await writeFile(
    path.join(projectDir, SID, 'subagents', 'agent-a1.jsonl'),
    line({ type: 'assistant', isSidechain: true, timestamp: '2026-10-07T09:59:03.000Z', message: { id: 's1', stop_reason: 'tool_use', usage: { input_tokens: 7, output_tokens: 2 }, content: [{ type: 'tool_use', id: 'toolu_grep', name: 'Grep' }] } }),
  )
}

export async function monitorScenario(): Promise<Trace> {
  const root = await realpath(await mkdtemp(path.join(os.tmpdir(), 'ccm-trace-root-')))
  const projectDir = path.join(root, 'projects', SLUG)
  const subDir = path.join(projectDir, SID, 'subagents')
  const main = path.join(projectDir, `${SID}.jsonl`)
  const regFile = path.join(root, 'sessions', `${PID}.json`)
  let now = Date.parse('2026-10-07T10:00:00.000Z')
  const wallStart = Date.now()
  await seed(root, projectDir)
  const store = new MonitorStore({ activityLimit: 300, batchMs: 1 })
  const monitor = new Monitor(config(root), store, { isPidAlive: () => true, queryProcesses: async () => null, now: () => now })
  const steps: TraceStep[] = []
  let current: TraceStep = { name: '', sent: [], frames: [] }
  store.subscribe((m) => current.frames.push(m))
  const reply = (event: MonitorEvent) => current.frames.push({ seq: -1, direct: true, events: [event] })
  const step = async (name: string, run: () => Promise<void> | void) => {
    current = { name, sent: [], frames: [] }
    steps.push(current)
    await run()
    store.flush()
  }
  const tid = `${PID}:134358400013081891`
  try {
    await step('boot', async () => {
      await monitor.start({ watch: false })
      current.frames.push(store.snapshotMessage())
    })
    await step('tool-result', async () => {
      await appendFile(main, line({ type: 'user', timestamp: '2026-10-07T10:00:00.000Z', message: { role: 'user', content: [{ type: 'tool_result', tool_use_id: 'toolu_bash', content: 'out' }] } }))
      await monitor.reconcile()
    })
    await step('toolSpans.get', () => {
      monitor.sendToolSpans(reply, tid)
      monitor.sendToolSpans(reply)
      monitor.sendToolSpans(reply, tid, '2026-10-07T10:00:00.500Z')
    })
    await step('alias-note', () => {
      monitor.setAlias(tid, '  Ads work  ')
      monitor.setNote(tid, 'remember the ads key')
      monitor.setAlias('nope', 'x')
    })
    await step('nested-subagent', async () => {
      await appendFile(
        path.join(subDir, 'agent-a1.jsonl'),
        line({ type: 'assistant', isSidechain: true, timestamp: '2026-10-07T09:59:04.000Z', message: { id: 's2', stop_reason: 'tool_use', content: [{ type: 'tool_use', id: 'toolu_nested', name: 'Agent', input: { prompt: 'p' } }] } }) +
          line({ type: 'user', isSidechain: true, timestamp: '2026-10-07T09:59:05.000Z', toolUseResult: { status: 'async_launched' }, message: { content: [{ type: 'tool_result', tool_use_id: 'toolu_nested' }] } }),
      )
      await writeFile(path.join(subDir, 'agent-a2.meta.json'), JSON.stringify({ agentType: 'Explore', description: 'nested', toolUseId: 'toolu_nested', parentAgentId: 'a1', spawnDepth: 2, requestShape: 'background' }))
      await writeFile(path.join(subDir, 'agent-a2.jsonl'), '')
      await monitor.reconcile()
    })
    await step('nested-failed', async () => {
      await appendFile(
        path.join(subDir, 'agent-a1.jsonl'),
        line({ type: 'user', isSidechain: true, timestamp: '2026-10-07T10:00:06.000Z', message: { content: '<task-notification><task-id>a2</task-id><tool-use-id>toolu_nested</tool-use-id><status>failed</status><summary>s</summary></task-notification>' } }),
      )
      await monitor.reconcile()
    })
    await step('subagent-completed', async () => {
      await appendFile(
        main,
        line({ type: 'queue-operation', operation: 'enqueue', timestamp: '2026-10-07T10:00:05.000Z', content: '<task-notification><task-id>a1</task-id><tool-use-id>toolu_agent</tool-use-id><status>completed</status><summary>s</summary></task-notification>' }),
      )
      await monitor.reconcile()
    })
    await step('rename-waiting', async () => {
      await writeFile(regFile, registry({ name: 'Android Ads', nameSource: 'user', status: 'waiting', waitingFor: 'permission prompt' }))
      await monitor.reconcile()
    })
    await step('shell', async () => {
      await writeFile(regFile, registry({ name: 'Android Ads', status: 'shell' }))
      await monitor.reconcile()
    })
    await step('clear', async () => {
      await writeFile(path.join(projectDir, `${SID2}.jsonl`), '')
      await writeFile(regFile, registry({ name: 'Android Ads', sessionId: SID2, status: 'idle' }))
      await monitor.reconcile()
    })
    await step('end', async () => {
      await rm(regFile)
      await monitor.reconcile()
      monitor.sendHistory(reply)
      monitor.sendTimeline(reply)
    })
    await step('stale-removal', async () => {
      now += 121 * 60_000
      await monitor.reconcile()
    })
    await step('final', () => {
      current.frames.push(store.snapshotMessage())
    })
  } finally {
    monitor.stop()
    await rm(root, { recursive: true, force: true })
  }
  return {
    format: 'ccm-trace/1',
    scenario: 'monitor-fixture',
    source: 'node',
    wall: { start: wallStart, end: Date.now() },
    pids: {},
    replace: [[root, '<root>']],
    ignore: [],
    steps: JSON.parse(JSON.stringify(steps)) as TraceStep[],
  }
}
