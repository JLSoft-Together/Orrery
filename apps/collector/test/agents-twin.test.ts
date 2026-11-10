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
process.env.CCM_CORE_CODEX = 'node'

interface CompareLib {
  compareTraces: (a: unknown, b: unknown, options?: { strict?: boolean; limit?: number }) => { diffs: string[]; steps: number; frames: number[] }
}

type Private = {
  sync(id: string, file: string): Promise<void>
  scanNow(): Promise<void>
  tick(): Promise<void>
  reloadNames(): Promise<void>
  readOutbox(catchUp: boolean): Promise<void>
}

const SID = '11111111-2222-3333-4444-555555555555'
const CLAUDE_PID = 100
const CLAUDE_START = '134358400013081891'
const CLAUDE_TID = `${CLAUDE_PID}:${CLAUDE_START}`
const T0 = Date.parse('2026-10-11T07:00:00.000Z')
const NOW = T0 + 10_000
const iso = (ms: number) => new Date(ms).toISOString()
const line = (o: unknown) => `${JSON.stringify(o)}\n`
const rec = (type: string, payload: Record<string, unknown>, at = T0) => ({ timestamp: iso(at), ordinal: 0, type, payload })
const usage = (input: number, cached: number, output: number) => ({ input_tokens: input, cached_input_tokens: cached, output_tokens: output, reasoning_output_tokens: 0, total_tokens: input + output })
const id = (n: number) => `01a125e6-0000-7000-8000-${String(n).padStart(12, '0')}`

const meta = (tid: string, over: Record<string, unknown> = {}, at = T0) =>
  rec('session_meta', { id: tid, session_id: tid, timestamp: iso(at), cwd: 'C:\\Temp\\art-hero-x1', originator: 'codex_exec', cli_version: '0.162.0', source: 'exec', base_instructions: 'secret '.repeat(40), ...over }, at)
const started = (at = T0) => rec('event_msg', { type: 'task_started', turn_id: 't1', model_context_window: 258400 }, at)
const context = (at = T0) => rec('turn_context', { turn_id: 't1', model: 'gpt-6-luna', effort: 'medium', approval_policy: 'never' }, at)
const call = (cid: string, at = T0) => rec('response_item', { type: 'custom_tool_call', call_id: cid, name: 'exec', input: 'secret command' }, at)
const execCall = (cid: string, cmd: string, at = T0) => rec('response_item', { type: 'function_call', call_id: cid, name: 'exec_command', arguments: JSON.stringify({ cmd }) }, at)
const output = (cid: string, at = T0) => rec('response_item', { type: 'custom_tool_call_output', call_id: cid, output: 'secret output' }, at)
const tokens = (at = T0, used = 7) =>
  rec(
    'event_msg',
    {
      type: 'token_count',
      info: { total_token_usage: usage(18000, 12000, 2000), last_token_usage: usage(18000, 12000, 2000), model_context_window: 258400 },
      rate_limits: { limit_id: 'codex', primary: { used_percent: used, window_minutes: 300, resets_at: 1_791_650_000 }, secondary: { used_percent: 12, window_minutes: 10080, resets_at: 1_792_000_000 }, plan_type: 'plus' },
    },
    at,
  )
const complete = (at = T0) => rec('event_msg', { type: 'task_complete', turn_id: 't1', last_agent_message: 'secret answer' }, at)
const message = (at = T0) => rec('response_item', { type: 'message', role: 'user', content: [{ type: 'input_text', text: 'secret prompt '.repeat(30) }] }, at)

function dayDirOf(home: string, ms: number): string {
  const d = new Date(ms)
  const pad = (n: number) => String(n).padStart(2, '0')
  return path.join(home, 'sessions', String(d.getFullYear()), pad(d.getMonth() + 1), pad(d.getDate()))
}

const proc = (pid: number, start: number, over: Record<string, unknown> = {}) => ({ pid, start, mode: 'exec', ancestors: [], orphan: false, ...over })

async function codexScenario(twin: MonitorTwin, root: string, codexHome: string): Promise<void> {
  const n = twin.node
  const codex = n.codex as unknown as Private
  const r = (cmd: Record<string, unknown>) => twin.rustCmd(cmd)
  const dir = dayDirOf(codexHome, NOW)
  const oldDir = path.join(codexHome, 'sessions', '2026', '09', '01')
  const file = (tid: string, folder = dir) => path.join(folder, `rollout-2026-10-11T14-00-00-${tid}.jsonl`)
  const T = { claude: id(1), alone: id(2), desktop: id(3), done: id(4), killed: id(5), sub: id(6), plugin: id(7), late: id(8), old: id(9), fresh: id(10) }
  const write = (tid: string, lines: unknown[], folder = dir) => writeFile(file(tid, folder), lines.map(line).join(''))
  const sync = async (tid: string, folder = dir) => {
    await twin.both(() => codex.sync(tid, file(tid, folder)), { cmd: 'codexSync', id: tid, file: file(tid, folder) })
  }

  const projectDir = path.join(root, 'projects', cwdSlug('D:\\work\\demo'))
  const subDir = path.join(projectDir, SID, 'subagents')
  await mkdir(path.join(root, 'sessions'), { recursive: true })
  await mkdir(subDir, { recursive: true })
  await mkdir(dir, { recursive: true })
  await mkdir(oldDir, { recursive: true })
  await writeFile(path.join(root, 'sessions', `${CLAUDE_PID}.json`), JSON.stringify({ pid: CLAUDE_PID, sessionId: SID, cwd: 'D:\\work\\demo', startedAt: 1_791_366_405_292, procStart: CLAUDE_START, version: '2.1.289', kind: 'interactive', entrypoint: 'cli', name: 'demo', status: 'busy', statusUpdatedAt: 1_791_366_405_292 }))
  await writeFile(
    path.join(projectDir, `${SID}.jsonl`),
    line({ type: 'assistant', timestamp: iso(T0 - 5_000), message: { id: 'm1', model: 'claude-opus-5-5', stop_reason: 'tool_use', usage: { input_tokens: 100, output_tokens: 20 }, content: [{ type: 'tool_use', id: 'toolu_agent', name: 'Agent', input: { prompt: 'p' } }] } }) +
      line({ type: 'user', timestamp: iso(T0 - 4_900), toolUseResult: { status: 'async_launched' }, message: { content: [{ type: 'tool_result', tool_use_id: 'toolu_agent' }] } }),
  )
  await writeFile(path.join(subDir, 'agent-a1.meta.json'), JSON.stringify({ agentType: 'Explore', description: 'Call codex', toolUseId: 'toolu_agent', spawnDepth: 1, requestShape: 'background' }))
  await writeFile(
    path.join(subDir, 'agent-a1.jsonl'),
    line({ type: 'assistant', isSidechain: true, timestamp: iso(T0 - 1_000), message: { id: 's1', stop_reason: 'tool_use', usage: { input_tokens: 7, output_tokens: 2 }, content: [{ type: 'tool_use', id: 'toolu_codex', name: 'Bash', input: { command: 'cat p.md | codex exec -s read-only -' } }] } }),
  )

  await write(T.claude, [meta(T.claude), started(), context(), call('c1', T0 + 1_000), tokens(T0 + 2_000, 7)])
  await write(T.alone, [meta(T.alone, { cwd: 'C:\\Temp\\solo' }, T0 + 1_500), started(T0 + 1_500), context(T0 + 1_500), message(T0 + 1_600), tokens(T0 + 1_700, 9)])
  await write(T.desktop, [meta(T.desktop, { source: 'vscode', originator: 'Codex Desktop' }), started()])
  await write(T.done, [meta(T.done, {}, T0 + 2_000), started(T0 + 2_000), tokens(T0 + 2_100, 33), complete(T0 + 2_200)])
  await write(T.killed, [meta(T.killed, {}, T0 + 2_500), started(T0 + 2_500), call('c1', T0 + 2_600)])
  await write(T.sub, [meta(T.sub, { source: { subagent: { thread_spawn: { parent_thread_id: T.claude, depth: 1 } } } }, T0 + 3_000), started(T0 + 3_000), context(T0 + 3_000), call('s1', T0 + 3_100)])
  await write(T.plugin, [meta(T.plugin, { source: 'vscode', originator: 'Claude Code', cwd: 'D:\\work\\demo' }, T0 + 4_000), started(T0 + 4_000), context(T0 + 4_000)])
  await writeFile(path.join(codexHome, 'session_index.jsonl'), line({ id: T.alone, thread_name: 'Solo run', updated_at: 'x' }))

  await twin.step('boot', async () => {
    await twin.setCodexProcs([proc(300, T0 - 200, { ancestors: [250, CLAUDE_PID] }), proc(400, T0 + 1_400, { ancestors: [1] })])
    await twin.both(() => n.start({ watch: false }), { cmd: 'start' })
    await twin.both(() => n.codex!.start({ watch: false }), { cmd: 'startAgents' })
    await twin.snapshot()
  })
  await twin.step('tool-started', async () => {
    await appendFile(file(T.claude), [output('c1', T0 + 3_000), call('c2', T0 + 3_100)].map(line).join(''))
    await sync(T.claude)
    await appendFile(file(T.alone), [execCall('x1', 'cargo test --token hunter22', T0 + 3_200)].map(line).join(''))
    await sync(T.alone)
  })
  await twin.step('names', async () => {
    await writeFile(path.join(codexHome, 'session_index.jsonl'), line({ id: T.alone, thread_name: 'Solo run renamed' }) + line({ id: T.claude, thread_name: 'Review ads' }))
    await twin.both(() => codex.reloadNames(), { cmd: 'codexNames' })
  })
  await twin.step('turn-complete', async () => {
    await appendFile(file(T.claude), [output('c2', T0 + 4_000), complete(T0 + 4_100)].map(line).join(''))
    await sync(T.claude)
    await appendFile(file(T.sub), [output('s1', T0 + 4_200), complete(T0 + 4_300)].map(line).join(''))
    await sync(T.sub)
  })
  await twin.step('new-thread', async () => {
    await write(T.late, [meta(T.late, { cwd: 'C:\\Temp\\late' }, T0 + 6_000), started(T0 + 6_000), context(T0 + 6_000)])
    await write(T.fresh, [meta(T.fresh, { cwd: 'C:\\Temp\\fresh' }, T0 + 6_200), started(T0 + 6_200), context(T0 + 6_200), tokens(T0 + 6_300, 15)])
    await twin.setCodexProcs([proc(300, T0 - 200, { ancestors: [250, CLAUDE_PID] }), proc(400, T0 + 1_400, { ancestors: [1] }), proc(450, T0 + 5_900, { ancestors: [60, 1], orphan: true }), proc(460, T0 + 6_100, { ancestors: [1] })])
    await sync(T.late)
    await sync(T.fresh)
    await twin.both(() => codex.scanNow(), { cmd: 'codexScan' })
  })
  await twin.step('resumed-old-thread', async () => {
    await write(T.old, [meta(T.old, { source: 'cli', originator: 'codex-tui', cwd: 'C:\\Temp\\old' }, T0 - 86_400_000 * 40), started(T0 + 7_000), context(T0 + 7_000), tokens(T0 + 7_100, 11)], oldDir)
    await twin.setCodexProcs([proc(300, T0 - 200, { ancestors: [250, CLAUDE_PID] }), proc(400, T0 + 1_400, { ancestors: [1] }), proc(450, T0 + 5_900, { ancestors: [60, 1], orphan: true }), proc(460, T0 + 6_100, { ancestors: [1] }), proc(500, T0 + 6_500, { mode: 'tui', resumeId: T.old, ancestors: [1] })])
    await twin.both(() => codex.scanNow(), { cmd: 'codexScan' })
  })
  await twin.step('process-gone', async () => {
    await twin.kill(400)
    await twin.kill(300)
    await twin.both(() => codex.tick(), { cmd: 'codexTick' })
  })
  await twin.step('diagnostics-and-info', async () => {
    const a = n.codex!.diagnostics()
    const b = (await twin.rustCmd({ cmd: 'diagnostics' })) as { codex: unknown }
    expect({ ...a, home: '<home>' }).toEqual({ ...(b.codex as object), home: '<home>' })
    for (const tid of [T.alone, T.late, T.old, T.claude]) {
      const info = (await twin.rustCmd({ cmd: 'agentInfo', terminalId: `codex:${tid}` })) as { pid: number | null; cwd: string | null; live: boolean }
      expect({ pid: info.pid ?? undefined, cwd: info.cwd ?? undefined, live: info.live }).toEqual({ pid: n.codex!.pidOf(`codex:${tid}`), cwd: n.codex!.cwdOf(`codex:${tid}`), live: n.codex!.isLive(`codex:${tid}`) })
    }
  })
  await twin.step('reopen', async () => {
    await twin.setCodexProcs([proc(600, T0 + 8_000, { mode: 'tui', ancestors: [1] })])
    await appendFile(file(T.alone), [started(T0 + 9_000), context(T0 + 9_000), call('y1', T0 + 9_100)].map(line).join(''))
    await sync(T.alone)
    await twin.both(() => codex.scanNow(), { cmd: 'codexScan' })
  })
  await twin.step('dismiss', async () => {
    await twin.both(() => n.dismissEnded(), { cmd: 'dismiss' })
  })
  await twin.step('stale-removal', async () => {
    await twin.setNow(NOW + 121 * 60_000)
    await twin.both(() => codex.tick(), { cmd: 'codexTick' })
    await twin.both(() => n.reconcile(), { cmd: 'reconcile' })
  })
  await twin.step('final', async () => {
    await twin.snapshot()
  })
}

async function deepseekScenario(twin: MonitorTwin, root: string, home: string): Promise<void> {
  const n = twin.node
  const ds = n.deepseek as unknown as Private & { resolveOutbox(): Promise<void> }
  const outbox = path.join(home, 'outbox.jsonl')
  const outbox2 = path.join(home, 'rotated', 'outbox2.jsonl')
  const ws = (name: string) => path.join(root, 'work', name)
  const t = (s: number) => iso(T0 + s * 1000)
  const ev = (event: string, thread: string, at: number, extra: Record<string, unknown> = {}, workspace?: string) =>
    line({ schema_version: 1, event, thread_id: thread, timestamp: t(at), payload: { ...(workspace ? { workspace } : {}), ...extra } })

  await mkdir(home, { recursive: true })
  await mkdir(path.dirname(outbox2), { recursive: true })
  await writeFile(path.join(home, 'config.toml'), `model = "x"\n[lifecycle_outbox]\npath = '${outbox}'\n`)
  await writeFile(
    outbox,
    ev('session_start', 'ds-1', 0, {}, ws('alpha')) + ev('turn_start', 'ds-1', 1, {}, ws('alpha')) + ev('subagent_spawn', 'ds-1', 2, { subagent: { id: 'sa1', name: 'explore' } }) + 'torn line\n' + ev('session_start', 'ds-gone', 0, {}, ws('gone')) + ev('turn_start', 'ds-gone', 1),
  )
  await mkdir(ws('alpha'), { recursive: true })

  await twin.step('ds-boot', async () => {
    await twin.setDeepseekProcs([{ pid: 700, start: T0 - 100, ancestors: [1], workspace: ws('alpha') }])
    await twin.both(() => n.deepseek!.start({ watch: false }), { cmd: 'startAgents' })
    await twin.snapshot()
  })
  await twin.step('ds-events', async () => {
    await appendFile(outbox, ev('subagent_complete', 'ds-1', 3, { subagent: { id: 'sa1' }, status: 'completed' }) + ev('turn_end', 'ds-1', 4, {}) + ev('subagent_spawn', 'ds-1', 5, { subagent: 'sa2' }))
    await twin.both(() => ds.readOutbox(false), { cmd: 'deepseekRead' })
    await appendFile(outbox, line({ schema_version: 1, event: 'turn_end', kind: 'turn.failed', thread_id: 'ds-1', timestamp: t(6) }) + ev('subagent_complete', 'ds-1', 7, { status: 'failed' }))
    await twin.both(() => ds.readOutbox(false), { cmd: 'deepseekRead' })
  })
  await twin.step('ds-new-session', async () => {
    await twin.setDeepseekProcs([{ pid: 700, start: T0 - 100, ancestors: [1], workspace: ws('alpha') }, { pid: 710, start: T0 + 9_000, ancestors: [1], workspace: ws('beta') }])
    await appendFile(outbox, ev('session_start', 'ds-2', 9, {}, ws('beta')) + ev('turn_start', 'ds-2', 10, {}, ws('beta')))
    await twin.both(() => ds.readOutbox(false), { cmd: 'deepseekRead' })
    await twin.both(() => ds.scanNow(), { cmd: 'deepseekScan' })
  })
  await twin.step('ds-end-and-resume', async () => {
    await appendFile(outbox, ev('session_end', 'ds-2', 20) + ev('session_end', 'ds-unknown', 21))
    await twin.both(() => ds.readOutbox(false), { cmd: 'deepseekRead' })
    await twin.setDeepseekProcs([{ pid: 700, start: T0 - 100, ancestors: [1], workspace: ws('alpha') }, { pid: 720, start: T0 + 30_000, ancestors: [1], resumeId: 'ds-2' }])
    await appendFile(outbox, ev('session_start', 'ds-2', 30) + ev('turn_start', 'ds-2', 31))
    await twin.both(() => ds.readOutbox(false), { cmd: 'deepseekRead' })
    await twin.both(() => ds.scanNow(), { cmd: 'deepseekScan' })
  })
  await twin.step('ds-process-gone', async () => {
    await twin.kill(700)
    await twin.both(() => ds.tick(), { cmd: 'deepseekTick' })
    const info = (await twin.rustCmd({ cmd: 'agentInfo', terminalId: 'deepseek:ds-2' })) as { pid: number | null; cwd: string | null }
    expect({ pid: info.pid ?? undefined, cwd: info.cwd ?? undefined }).toEqual({ pid: n.deepseek!.pidOf('deepseek:ds-2'), cwd: n.deepseek!.cwdOf('deepseek:ds-2') })
    const a = n.deepseek!.diagnostics()
    const b = (await twin.rustCmd({ cmd: 'diagnostics' })) as { deepseek: unknown }
    expect(b.deepseek).toEqual(a)
  })
  await twin.step('ds-rotate-config', async () => {
    await writeFile(path.join(home, 'config.toml'), `[lifecycle_outbox]\npath = "${outbox2.replaceAll('\\', '\\\\')}"\n`)
    await writeFile(outbox2, ev('session_start', 'ds-3', 40, {}, ws('gamma')) + ev('turn_start', 'ds-3', 41))
    await twin.setDeepseekProcs([{ pid: 730, start: T0 + 40_000, ancestors: [1], workspace: ws('gamma') }])
    await twin.both(() => ds.resolveOutbox(), { cmd: 'deepseekResolve' })
    await twin.both(() => ds.readOutbox(false), { cmd: 'deepseekRead' })
    await twin.both(() => ds.scanNow(), { cmd: 'deepseekScan' })
  })
  await twin.step('ds-dismiss', async () => {
    await twin.both(() => n.dismissEnded(), { cmd: 'dismiss' })
  })
  await twin.step('ds-stale', async () => {
    await twin.setNow(NOW + 121 * 60_000)
    await twin.both(() => ds.tick(), { cmd: 'deepseekTick' })
  })
  await twin.step('ds-final', async () => {
    await twin.snapshot()
  })
}

let compare: CompareLib

beforeAll(async () => {
  compare = (await import(pathToFileURL(path.resolve(__dirname, '../../../scripts/trace-compare.mjs')).href)) as CompareLib
})

describe.skipIf(!SCRIPT_BIN)('Rust agent monitors twin (Codex + DeepSeek vs Node)', () => {
  const roots: string[] = []
  afterAll(async () => {
    for (const r of roots) await rm(r, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 })
  })

  const run = async (name: string, scenario: (twin: MonitorTwin, root: string, home: string) => Promise<void>): Promise<{ node: Trace; rust: Trace }> => {
    const root = await realpath(await mkdtemp(path.join(os.tmpdir(), `ccm-agents-${name}-`)))
    roots.push(root)
    const codexHome = path.join(root, 'codex')
    const codewhaleHome = path.join(root, 'codewhale')
    const twin = new MonitorTwin(SCRIPT_BIN!, { root, now: NOW, agents: { codexHome, codewhaleHome, homeDir: root } })
    try {
      await twin.init()
      await scenario(twin, root, name === 'codex' ? codexHome : codewhaleHome)
    } finally {
      twin.close()
    }
    const node = twin.trace('node')
    const rust = twin.trace('rust')
    const out = process.env.CCM_TRACE_OUT
    if (out) {
      await mkdir(out, { recursive: true })
      await writeFile(path.join(out, `agents-${name}-node.trace.json`), JSON.stringify(node))
      await writeFile(path.join(out, `agents-${name}-rust.trace.json`), JSON.stringify(rust))
    }
    return { node, rust }
  }

  const kindsOf = (t: Trace) => t.steps.flatMap((s) => s.frames.flatMap((f) => f.events.map((e) => (e.type === 'activity' ? `activity:${e.payload.kind}` : e.type))))

  it('gives the same frames for Codex runs linked to a Claude session, standalone, resumed and ended', async () => {
    const { node, rust } = await run('codex', codexScenario)
    expect(compare.compareTraces(node, rust).diffs).toEqual([])
    expect(compare.compareTraces(node, rust, { strict: true }).diffs).toEqual([])
    expect(JSON.stringify(rust)).toContain('"currentToolSummary":"cargo test --token [masked]"')
    expect(JSON.stringify([node, rust])).not.toContain('hunter22')
    const kinds = kindsOf(node)
    for (const k of ['codexLimits.updated', 'activity:tool.started', 'activity:agent.started', 'activity:agent.completed', 'activity:terminal.started', 'activity:terminal.ended', 'agent.created', 'agent.removed', 'terminal.created', 'terminal.removed']) expect(kinds).toContain(k)
    const agents = node.steps.flatMap((s) => s.frames.flatMap((f) => f.events.filter((e) => e.type === 'agent.created').map((e) => e.payload as unknown as Record<string, unknown>)))
    expect(agents.some((a) => a.terminalId === CLAUDE_TID && a.provider === 'codex' && typeof a.parentId === 'string')).toBe(true)
    expect(agents.some((a) => a.provider === 'codex' && a.parentId === `codex:${id(1)}`)).toBe(true)
  }, 120_000)

  it('gives the same frames for DeepSeek sessions: outbox events, processes, resume, rotated outbox', async () => {
    const { node, rust } = await run('deepseek', deepseekScenario)
    expect(compare.compareTraces(node, rust).diffs).toEqual([])
    expect(compare.compareTraces(node, rust, { strict: true }).diffs).toEqual([])
    const kinds = kindsOf(node)
    for (const k of ['activity:turn.started', 'activity:turn.completed', 'activity:turn.failed', 'activity:agent.started', 'activity:agent.completed', 'activity:agent.failed', 'activity:terminal.started', 'activity:terminal.ended', 'terminal.created', 'agent.created']) expect(kinds).toContain(k)
  }, 120_000)
})
