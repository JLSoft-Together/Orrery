import { execFileSync } from 'node:child_process'
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

const SID = '11111111-2222-3333-4444-555555555555'
const SID2 = '66666666-7777-8888-9999-000000000000'
const PID = 4242
const START = '134358400013081891'
const TID = `${PID}:${START}`
const SID3 = '77777777-8888-9999-aaaa-bbbbbbbbbbbb'
const PID2 = 4343
const START2 = '134358400013099999'
const line = (o: unknown) => `${JSON.stringify(o)}\n`
const T0 = Date.parse('2026-10-07T10:00:00.000Z')

function hasGit(): boolean {
  try {
    execFileSync('git', ['--version'], { stdio: 'ignore', windowsHide: true })
    return true
  } catch {
    return false
  }
}

const git = (cwd: string, ...args: string[]) => execFileSync('git', ['-C', cwd, ...args], { stdio: 'ignore', windowsHide: true, env: { ...process.env, GIT_TERMINAL_PROMPT: '0' } })

async function scenario(twin: MonitorTwin, root: string, cwd: string, withGit: boolean): Promise<void> {
  const projectDir = path.join(root, 'projects', cwdSlug(cwd))
  const subDir = path.join(projectDir, SID, 'subagents')
  const main = path.join(projectDir, `${SID}.jsonl`)
  const regFile = path.join(root, 'sessions', `${PID}.json`)
  const registry = (over: Record<string, unknown> = {}) =>
    JSON.stringify({ pid: PID, sessionId: SID, cwd, startedAt: 1_791_366_405_292, procStart: START, version: '2.1.289', kind: 'interactive', entrypoint: 'cli', name: 'demo-1a', nameSource: 'derived', status: 'busy', statusUpdatedAt: 1_791_366_405_292, messagingSocketPath: '\\\\.\\pipe\\LOCAL\\cc-msg-secret', ...over })
  const n = twin.node
  const r = (cmd: Record<string, unknown>) => twin.rustCmd(cmd)

  await mkdir(path.join(root, 'sessions'), { recursive: true })
  await mkdir(subDir, { recursive: true })
  await writeFile(regFile, registry())
  await writeFile(
    main,
    line({ type: 'assistant', timestamp: '2026-10-07T09:59:00.000Z', message: { id: 'm1', model: 'claude-opus-5-5', stop_reason: 'tool_use', usage: { input_tokens: 100, output_tokens: 20 }, content: [{ type: 'tool_use', id: 'toolu_agent', name: 'Agent', input: { prompt: 'p' } }] } }) +
      line({ type: 'user', timestamp: '2026-10-07T09:59:01.000Z', toolUseResult: { status: 'async_launched' }, message: { content: [{ type: 'tool_result', tool_use_id: 'toolu_agent' }] } }) +
      line({ type: 'assistant', timestamp: '2026-10-07T09:59:02.000Z', message: { id: 'm2', stop_reason: 'tool_use', usage: { input_tokens: 50, output_tokens: 5 }, content: [{ type: 'tool_use', id: 'toolu_bash', name: 'Bash', input: { command: 'echo' } }] } }),
  )
  await writeFile(path.join(subDir, 'agent-a1.meta.json'), JSON.stringify({ agentType: 'Explore', description: 'Find ads code', toolUseId: 'toolu_agent', spawnDepth: 1, requestShape: 'background' }))
  await writeFile(path.join(subDir, 'agent-a1.jsonl'), line({ type: 'assistant', isSidechain: true, timestamp: '2026-10-07T09:59:03.000Z', message: { id: 's1', stop_reason: 'tool_use', usage: { input_tokens: 7, output_tokens: 2 }, content: [{ type: 'tool_use', id: 'toolu_grep', name: 'Grep' }] } }))

  await twin.step('boot', async () => {
    await n.start({ watch: false })
    await r({ cmd: 'start' })
    if (withGit) await twin.wait()
    await twin.snapshot()
  })
  await twin.step('tool-result', async () => {
    await appendFile(main, line({ type: 'user', timestamp: '2026-10-07T10:00:00.000Z', message: { role: 'user', content: [{ type: 'tool_result', tool_use_id: 'toolu_bash', content: 'out' }] } }))
    await twin.both(() => n.reconcile(), { cmd: 'reconcile' })
  })
  await twin.step('toolSpans.get', async () => {
    await twin.both(() => n.sendToolSpans(twin.reply, TID), { cmd: 'toolSpans', terminalId: TID })
    await twin.both(() => n.sendToolSpans(twin.reply), { cmd: 'toolSpans' })
    await twin.both(() => n.sendToolSpans(twin.reply, TID, '2026-10-07T10:00:00.500Z'), { cmd: 'toolSpans', terminalId: TID, since: '2026-10-07T10:00:00.500Z' })
  })
  await twin.step('alias-note', async () => {
    expect(await twin.both(() => n.setAlias(TID, '  Ads work  '), { cmd: 'alias', terminalId: TID, alias: '  Ads work  ' })).toEqual([true, true])
    await twin.both(() => n.setNote(TID, 'remember the ads key'), { cmd: 'note', terminalId: TID, note: 'remember the ads key' })
    expect(await twin.both(() => n.setAlias('nope', 'x'), { cmd: 'alias', terminalId: 'nope', alias: 'x' })).toEqual([false, false])
  })
  await twin.step('nested-subagent', async () => {
    await appendFile(
      path.join(subDir, 'agent-a1.jsonl'),
      line({ type: 'assistant', isSidechain: true, timestamp: '2026-10-07T09:59:04.000Z', message: { id: 's2', stop_reason: 'tool_use', content: [{ type: 'tool_use', id: 'toolu_nested', name: 'Agent', input: { prompt: 'p' } }] } }) +
        line({ type: 'user', isSidechain: true, timestamp: '2026-10-07T09:59:05.000Z', toolUseResult: { status: 'async_launched' }, message: { content: [{ type: 'tool_result', tool_use_id: 'toolu_nested' }] } }),
    )
    await writeFile(path.join(subDir, 'agent-a2.meta.json'), JSON.stringify({ agentType: 'Explore', description: 'nested', toolUseId: 'toolu_nested', parentAgentId: 'a1', spawnDepth: 2, requestShape: 'background' }))
    await writeFile(path.join(subDir, 'agent-a2.jsonl'), '')
    await twin.both(() => n.reconcile(), { cmd: 'reconcile' })
  })
  await twin.step('nested-failed', async () => {
    await appendFile(path.join(subDir, 'agent-a1.jsonl'), line({ type: 'user', isSidechain: true, timestamp: '2026-10-07T10:00:06.000Z', message: { content: '<task-notification><task-id>a2</task-id><tool-use-id>toolu_nested</tool-use-id><status>failed</status><summary>s</summary></task-notification>' } }))
    await twin.both(() => n.reconcile(), { cmd: 'reconcile' })
  })
  await twin.step('subagent-completed', async () => {
    await appendFile(main, line({ type: 'queue-operation', operation: 'enqueue', timestamp: '2026-10-07T10:00:05.000Z', content: '<task-notification><task-id>a1</task-id><tool-use-id>toolu_agent</tool-use-id><status>completed</status><summary>s</summary></task-notification>' }))
    await twin.both(() => n.reconcile(), { cmd: 'reconcile' })
  })
  await twin.step('context-hints', async () => {
    const doc = { surfaces: { cc: { model_selector_config: [{ models: [{ id: 'claude-opus-5-5', runtime: { max_input_tokens: 1_000_000 } }] }] } } }
    await mkdir(path.join(root, 'cache', 'model-catalog'), { recursive: true })
    await writeFile(path.join(root, 'cache', 'model-catalog', 'published-x.json'), JSON.stringify({ documentBytes: Buffer.from(JSON.stringify(doc)).toString('base64'), catalog: { config: { models: [{ id: 'claude-opus-5-5', name: 'Opus 5.5' }] } } }))
    await writeFile(path.join(root, 'settings.json'), JSON.stringify({ model: 'opus[1m]' }))
    await twin.both(() => (n as unknown as { refreshHints: () => Promise<void> }).refreshHints(), { cmd: 'refreshHints' })
  })
  await twin.step('hooks-denied', async () => {
    await mkdir(path.join(root, 'hooks'), { recursive: true })
    await writeFile(path.join(root, 'hooks', `${SID}.json`), JSON.stringify({ sessionId: SID, at: '2026-10-07T10:00:07.000Z', deniedCount: 2, deniedTools: ['Bash', 'Edit', 'bad tool!'], compactingSince: '2026-10-07T09:58:00.000Z', compactTrigger: 'manual' }))
    await twin.both(() => n.pollExtras(), { cmd: 'pollExtras' })
    await twin.both(() => n.pollExtras(), { cmd: 'pollExtras' })
    await twin.both(() => n.setNote(TID, 'denied twice'), { cmd: 'note', terminalId: TID, note: 'denied twice' })
  })
  await twin.step('statusline-limits', async () => {
    await mkdir(path.join(root, 'statusline'), { recursive: true })
    await writeFile(path.join(root, 'statusline', `${SID}.json`), JSON.stringify({ sessionId: SID, at: '2026-10-07T10:00:08.000Z', costUsd: 1.25, contextWindow: 500_000, contextPct: 12.5, bufferPct: 10, fiveHour: { usedPct: 41, resetsAt: '2026-10-07T12:00:00.000Z' }, sevenDay: { usedPct: 7 } }))
    await twin.both(() => n.pollExtras(), { cmd: 'pollExtras' })
  })
  await twin.step('limit-hit', async () => {
    await appendFile(main, line({ type: 'assistant', timestamp: '2026-10-07T10:00:09.000Z', isApiErrorMessage: true, apiError: 'usage_limit_reached', quotaLimits: { resetsAt: 1_791_380_000, rateLimitType: 'five_hour' }, message: { id: 'm3', model: '<synthetic>', content: [{ type: 'text', text: 'limit' }], stop_reason: 'stop_sequence' } }))
    await twin.both(() => n.reconcile(), { cmd: 'reconcile' })
    await appendFile(main, line({ type: 'system', subtype: 'informational', timestamp: '2026-10-07T10:00:10.000Z', content: 'note' }))
    await twin.both(() => n.reconcile(), { cmd: 'reconcile' })
    await twin.both(() => n.setAlias(TID, 'Ads work 2'), { cmd: 'alias', terminalId: TID, alias: 'Ads work 2' })
  })
  await twin.step('jobs', async () => {
    const dir = path.join(root, 'jobs', 'job-0001')
    await mkdir(dir, { recursive: true })
    await writeFile(path.join(dir, 'state.json'), JSON.stringify({ name: 'Nightly', state: 'working', inFlight: { tasks: 1 }, intent: 'secret' }))
    await writeFile(path.join(dir, 'timeline.jsonl'), line({ at: '2026-10-07T10:00:00.000Z', state: 'working' }))
    await twin.both(() => n.pollExtras(), { cmd: 'pollExtras' })
    await new Promise((res) => setTimeout(res, 20))
    await writeFile(path.join(dir, 'state.json'), JSON.stringify({ name: 'Nightly', state: 'blocked' }))
    await twin.both(() => n.pollExtras(), { cmd: 'pollExtras' })
    expect(await twin.both(() => n.stopJob('job-0001'), { cmd: 'stopJob', jobId: 'job-0001' })).toEqual(['ok', true])
    await twin.rustCmd({ cmd: 'wait', ms: 5_000 })
  })
  await twin.step('rename-waiting', async () => {
    await writeFile(regFile, registry({ name: 'Android Ads', nameSource: 'user', status: 'waiting', waitingFor: 'permission prompt', statusUpdatedAt: 1_791_366_500_000 }))
    await twin.both(() => n.reconcile(), { cmd: 'reconcile' })
    if (withGit) await twin.wait()
  })
  await twin.step('git-diff', async () => {
    if (withGit) {
      await writeFile(path.join(cwd, 'a.txt'), 'one\ntwo\nthree\n')
      await writeFile(path.join(cwd, 'new.txt'), 'x\n')
    }
    await writeFile(regFile, registry({ name: 'Android Ads', status: 'idle', statusUpdatedAt: 1_791_366_600_000 }))
    await twin.both(() => n.reconcile(), { cmd: 'reconcile' })
    if (withGit) await twin.wait()
    await writeFile(regFile, registry({ name: 'Android Ads', status: 'shell', statusUpdatedAt: 1_791_366_700_000 }))
    await twin.both(() => n.reconcile(), { cmd: 'reconcile' })
    if (withGit) await twin.wait()
  })
  const reg2 = path.join(root, 'sessions', `${PID2}.json`)
  const tid2 = `${PID2}:${START2}`
  await twin.step('second-session', async () => {
    const dir2 = path.join(root, 'projects', cwdSlug('D:\\work\\other'))
    await mkdir(path.join(dir2, SID3, 'subagents'), { recursive: true })
    await writeFile(
      path.join(dir2, `${SID3}.jsonl`),
      line({ type: 'assistant', timestamp: '2026-10-07T10:01:00.000Z', message: { id: 'f1', model: 'claude-sonnet-5', stop_reason: 'tool_use', usage: { input_tokens: 9, output_tokens: 3 }, content: [{ type: 'tool_use', id: 'toolu_fg', name: 'Agent', input: { prompt: 'p' } }] } }),
    )
    await writeFile(path.join(dir2, SID3, 'subagents', 'agent-f1.meta.json'), JSON.stringify({ agentType: 'Plan', description: 'foreground', toolUseId: 'toolu_fg', spawnDepth: 1 }))
    await writeFile(path.join(dir2, SID3, 'subagents', 'agent-f1.jsonl'), line({ type: 'assistant', isSidechain: true, timestamp: '2026-10-07T10:01:01.000Z', message: { id: 'g1', stop_reason: 'tool_use', usage: { input_tokens: 4, output_tokens: 1 }, content: [{ type: 'tool_use', id: 'toolu_read', name: 'Read' }] } }))
    await writeFile(reg2, JSON.stringify({ pid: PID2, sessionId: SID3, cwd: 'D:\\work\\other', startedAt: 1_791_366_405_999, procStart: START2, status: 'busy', statusUpdatedAt: 1_791_366_405_999 }))
    await twin.both(() => n.reconcile(), { cmd: 'reconcile' })
    await appendFile(path.join(dir2, `${SID3}.jsonl`), line({ type: 'user', timestamp: '2026-10-07T10:01:05.000Z', toolUseResult: { status: 'completed' }, message: { content: [{ type: 'tool_result', tool_use_id: 'toolu_fg', content: 'ok' }] } }))
    await twin.both(() => n.reconcile(), { cmd: 'reconcile' })
  })
  await twin.step('favorites', async () => {
    await twin.both(() => n.toggleFavorite(tid2), { cmd: 'toggleFavorite', terminalId: tid2 })
    await twin.both(() => n.addFavorite(root, null), { cmd: 'addFavorite', dir: root, label: null })
    await twin.both(() => n.addFavorite(root, 'again'), { cmd: 'addFavorite', dir: root, label: 'again' })
    await twin.both(() => n.addFavorite('relative/dir', null), { cmd: 'addFavorite', dir: 'relative/dir', label: null })
    await twin.both(() => n.addFavorite(path.join(root, 'missing'), null), { cmd: 'addFavorite', dir: path.join(root, 'missing'), label: null })
  })
  await twin.step('dismiss-second', async () => {
    await twin.kill(PID2)
    await twin.both(() => n.reconcile(), { cmd: 'reconcile' })
    await rm(reg2)
    expect(await twin.both(() => n.dismissEnded(TID), { cmd: 'dismiss', terminalId: TID })).toEqual([0, 0])
    expect(await twin.both(() => n.dismissEnded(tid2), { cmd: 'dismiss', terminalId: tid2 })).toEqual([1, 1])
  })
  await twin.step('clear', async () => {
    await writeFile(path.join(projectDir, `${SID2}.jsonl`), '')
    await writeFile(regFile, registry({ name: 'Android Ads', sessionId: SID2, status: 'idle' }))
    await twin.both(() => n.reconcile(), { cmd: 'reconcile' })
  })
  await twin.step('end', async () => {
    await rm(regFile)
    await twin.both(() => n.reconcile(), { cmd: 'reconcile' })
    await twin.both(() => n.sendHistory(twin.reply), { cmd: 'history' })
    await twin.both(() => n.sendTimeline(twin.reply), { cmd: 'timeline' })
  })
  await twin.step('stale-removal', async () => {
    await twin.setNow(T0 + 121 * 60_000)
    await twin.both(() => n.reconcile(), { cmd: 'reconcile' })
  })
  await twin.step('final', async () => {
    await twin.snapshot()
  })
}

let compare: CompareLib

beforeAll(async () => {
  compare = (await import(pathToFileURL(path.resolve(__dirname, '../../../scripts/trace-compare.mjs')).href)) as CompareLib
})

describe.skipIf(!SCRIPT_BIN)('Rust monitor twin (core-monitor vs Node Monitor on one fixture)', () => {
  const roots: string[] = []
  afterAll(async () => {
    for (const r of roots) await rm(r, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 })
  })

  for (const withGit of [false, true]) {
    it.skipIf(withGit && !hasGit())(`gives the same frames${withGit ? ' with a git diff' : ''}`, async () => {
      const root = await realpath(await mkdtemp(path.join(os.tmpdir(), 'ccm-twin-')))
      roots.push(root)
      const cwd = withGit ? path.join(root, 'repo') : 'D:\\work\\demo'
      if (withGit) {
        await mkdir(cwd, { recursive: true })
        git(cwd, 'init', '-q')
        await writeFile(path.join(cwd, 'a.txt'), 'one\n')
        git(cwd, 'add', '.')
        git(cwd, '-c', 'user.email=t@t', '-c', 'user.name=t', 'commit', '-q', '-m', 'init')
      }
      const twin = new MonitorTwin(SCRIPT_BIN!, { root, now: T0, diff: withGit })
      try {
        await twin.init()
        await scenario(twin, root, cwd, withGit)
      } finally {
        twin.close()
      }
      const node: Trace = twin.trace('node')
      const rust: Trace = twin.trace('rust')
      const out = process.env.CCM_TRACE_OUT
      if (out) {
        await mkdir(out, { recursive: true })
        await writeFile(path.join(out, `twin${withGit ? '-git' : ''}-node.trace.json`), JSON.stringify(node))
        await writeFile(path.join(out, `twin${withGit ? '-git' : ''}-rust.trace.json`), JSON.stringify(rust))
      }
      expect(compare.compareTraces(node, rust).diffs).toEqual([])
      expect(compare.compareTraces(node, rust, { strict: true }).diffs).toEqual([])
      const kinds = node.steps.flatMap((s) => s.frames.flatMap((f) => f.events.map((e) => (e.type === 'activity' ? `activity:${e.payload.kind}` : e.type))))
      for (const k of ['activity:job.blocked', 'activity:job.stopped', 'limits.updated', 'models.updated', 'history.added', 'tool.span', 'activity:agent.failed', 'activity:session.cleared', 'activity:terminal.started', 'activity:agent.completed', 'favorites.updated', 'favorite.rejected', 'terminal.removed']) expect(kinds).toContain(k)
      const terminalPatches = node.steps.flatMap((s) => s.frames.flatMap((f) => f.events.filter((e) => e.type === 'terminal.updated' || e.type === 'terminal.created').map((e) => e.payload as Record<string, unknown>)))
      expect(terminalPatches.some((p) => Array.isArray(p.deniedTools))).toBe(true)
      expect(terminalPatches.some((p) => p.limitHit && typeof p.limitHit === 'object')).toBe(true)
      if (withGit) expect(terminalPatches.some((p) => p.diff && typeof p.diff === 'object')).toBe(true)
    }, 120_000)
  }
})
