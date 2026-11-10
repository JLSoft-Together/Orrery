import { existsSync, lstatSync, mkdirSync, mkdtempSync, readdirSync, rmSync, symlinkSync, utimesSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import type { AgentDataProgress } from '@ccm/shared'
import { registerAgentDataIpc, type AgentDataDeskDeps } from '../src/agentData/ipc'
import { MoveStore, freeBytes, isLocalDir } from '../src/agentData/move'
import { agentProcs } from '../src/agentData/procs'
import { AgentDataService, type AgentDataTexts, type ConfirmSpec } from '../src/agentData/service'
import { fakeIpc, HAVE_DESK, listTree, pinTimes, startTwin, swapRoot, type Twin } from './desk-twin'

const tmp = mkdtempSync(path.join(os.tmpdir(), 'ccm-desk-agentdata-'))
const procsFile = path.join(tmp, 'procs.json')
let twin: Twin

const SID = '11111111-2222-4333-8444-555555555555'
const LIVE = '22222222-2222-4333-8444-555555555555'
const SEP = '33333333-2222-4333-8444-555555555555'
const B64 = 'iVBORw0KGgo'.padEnd(4000, 'A')

const TEXTS: AgentDataTexts = {
  cancel: 'Cancel',
  group: (id) => `G(${id})`,
  cleanTitle: (files, size) => `clean ${files} ${size}`,
  cleanDetail: (lines, days) => `${lines.join('\n')}|${days}`,
  cleanConfirm: 'Delete',
  stripTitle: (files, images) => `strip ${files} ${images}`,
  stripDetail: (size, backup) => `${size}|${backup}`,
  stripConfirm: 'Strip',
  moveTitle: (from, to) => `move ${from} ${to}`,
  moveDetail: (size, original) => `${size}|${original}`,
  moveConfirm: 'Move',
  undoTitle: (folder) => `undo ${folder}`,
  undoDetail: (target, n, gone) => `${target}|${n}|${gone}`,
  undoCopyBack: (n) => `back ${n}`,
  undoOnly: 'Undo',
  deleteTitle: (folder) => `delete ${folder}`,
  deleteDetail: (size) => size,
  deleteConfirm: 'Delete original',
  discardTitle: (folder) => `discard ${folder}`,
  discardDetail: 'partial',
  discardConfirm: 'Delete copy',
}

interface Side {
  root: string
  claude: string
  codex: string
  ipc: ReturnType<typeof fakeIpc>
  confirms: ConfirmSpec[]
  answer: number
  events: AgentDataProgress[]
  collector: string[]
}

let node: Side
let rust: Side

function put(file: string, text: string): void {
  mkdirSync(path.dirname(file), { recursive: true })
  writeFileSync(file, text)
}

function seed(s: Side): void {
  const odd = { n: 1.0, big: 12345678901234567890, e: 1e-7, u: 'é\u0001\u007f😀', '2': 'two', b: true }
  const pasted = { type: 'user', sessionId: SID, extra: odd, message: { role: 'user', content: [{ type: 'text', text: 'look' }, { type: 'image', source: { type: 'base64', media_type: 'image/png', data: B64 } }] } }
  const readResult = {
    type: 'user',
    sessionId: SID,
    message: { role: 'user', content: [{ type: 'tool_result', tool_use_id: 'toolu_1', content: [{ type: 'image', source: { type: 'base64', media_type: 'image/jpeg', data: B64 } }] }] },
    toolUseResult: { type: 'image', file: { base64: B64, type: 'image/jpeg', originalSize: 3000 } },
  }
  const assistant = { type: 'assistant', message: { id: 'm1', role: 'assistant', content: [{ type: 'tool_use', id: 'toolu_1', name: 'Read', input: { file_path: 'a.png' } }] } }
  const lines = [JSON.stringify(assistant), JSON.stringify(pasted), JSON.stringify(readResult), 'not json "base64"', ''].join('\n')
  put(path.join(s.claude, 'projects', 'slug', `${SID}.jsonl`), `${lines}\n`)
  put(path.join(s.claude, 'projects', 'slug', `${SID}`, 'subagents', 'a.jsonl'), `${JSON.stringify(pasted)}\n`)
  put(path.join(s.claude, 'projects', 'slug', `${LIVE}.jsonl`), `${JSON.stringify(pasted)}\n`)
  put(path.join(s.claude, 'projects', 'slug', 'plain.jsonl'), '{"a":1}\n')
  put(path.join(s.claude, 'projects', 'slug', `${SEP}.jsonl`), `${JSON.stringify({ ...pasted, sessionId: SEP, sep: 'a\u2028b' })}\n`)
  put(path.join(s.claude, 'debug', 'a.txt'), 'd'.repeat(300))
  put(path.join(s.claude, 'debug', 'deep', 'b.txt'), 'b'.repeat(20))
  put(path.join(s.claude, 'shell-snapshots', 's.sh'), 'echo')
  put(path.join(s.claude, 'cache', 'c.bin'), 'c'.repeat(64))
  put(path.join(s.claude, 'settings.json'), '{}')
  put(path.join(s.claude, 'mystery.txt'), 'm')
  put(path.join(s.codex, 'log', 'codex.log'), 'l'.repeat(500))
  put(path.join(s.codex, 'sandbox.1.log'), 'sb')
  put(path.join(s.codex, 'sessions', '2026', 'r.jsonl'), '{"x":1}\n')
  put(path.join(s.codex, 'config.toml'), 'model = "x"\n')
  put(path.join(s.root, 'outside', 'keep.txt'), 'never touched')
  put(path.join(s.claude, 'sessions', `${process.pid}.json`), JSON.stringify({ pid: process.pid, sessionId: LIVE }))
  pinTimes(s.root)
  symlinkSync(path.join(s.root, 'outside'), path.join(s.claude, 'linked'))
  for (const d of ['target', 'backup']) mkdirSync(path.join(s.root, d), { recursive: true })
}

function side(name: string, withDesk: boolean): Side {
  const root = path.join(tmp, name)
  const s: Side = { root, claude: path.join(root, 'claude'), codex: path.join(root, 'codex'), ipc: fakeIpc(), confirms: [], answer: 0, events: [], collector: [] }
  const deps: AgentDataDeskDeps = {
    confirm: async (spec) => {
      s.confirms.push(spec)
      return s.answer
    },
    pickFolder: async (purpose) => path.join(root, purpose),
    stopCollector: async () => void s.collector.push('stop'),
    startCollector: async () => void s.collector.push('start'),
    texts: () => TEXTS,
  }
  const service = new AgentDataService({
    ...deps,
    roots: () => ({ claude: s.claude, codex: s.codex, claudeApp: null }),
    store: new MoveStore(path.join(tmp, `${name}-data`, 'agent-data-move.json')),
    procs: (agent) => agentProcs(agent),
    emit: (p) => s.events.push(p),
    isLocal: (dir) => isLocalDir(dir),
    freeBytes: (dir) => freeBytes(dir),
  })
  registerAgentDataIpc({ ipcMain: s.ipc.ipcMain, isAppFrame: (e) => (e as unknown) === 'app', service, desk: withDesk ? { router: twin.router, deps } : undefined })
  return s
}

const norm = (v: unknown): unknown =>
  JSON.parse(
    JSON.stringify(swapRoot(v, rust.root, node.root))
      .replace(/orrery-moved-\d+/g, 'orrery-moved-T')
      .replace(/orrery-transcripts-\d{4}-\d{2}-\d{2}-\d+/g, 'orrery-transcripts-T')
      .replace(/"at":\d+/g, '"at":0'),
  ) as unknown

const same = (a: unknown, b: unknown): void => expect(norm(b)).toEqual(norm(a))

async function both(channel: string, ...args: unknown[]): Promise<[unknown, unknown]> {
  const a = await node.ipc.call(channel, 'app', ...args)
  const b = await rust.ipc.call(channel, 'app', ...args)
  return [a, b]
}

async function twinCall(channel: string, ...args: unknown[]): Promise<unknown> {
  const [a, b] = await both(channel, ...args)
  same(a, b)
  return a
}

const tree = (s: Side, rel: string) => norm(swapRoot(listTree(path.join(s.root, rel)), s.root, node.root))

describe.skipIf(!HAVE_DESK)('agent data: TS handler and orrery-core desk give the same answers', () => {
  beforeAll(async () => {
    writeFileSync(procsFile, '[]')
    process.env.CCM_E2E_AGENT_PROCS = procsFile
    twin = await startTwin({
      CCM_DATA_DIR: path.join(tmp, 'rust-data'),
      CLAUDE_CONFIG_DIR: path.join(tmp, 'rust', 'claude'),
      CODEX_HOME: path.join(tmp, 'rust', 'codex'),
      CCM_E2E_AGENT_TARGET: path.join(tmp, 'rust', 'target'),
      CCM_E2E_AGENT_PROCS: procsFile,
    })
    node = side('node', false)
    rust = side('rust', true)
    seed(node)
    seed(rust)
  }, 60_000)

  afterAll(async () => {
    delete process.env.CCM_E2E_AGENT_PROCS
    await twin?.host.stop()
    rmSync(tmp, { recursive: true, force: true })
  })

  it('state, cancel and bad input match', async () => {
    const st = (await twinCall('ccm:agentdata-state')) as { places: unknown[] }
    expect(st.places).toHaveLength(2)
    await twinCall('ccm:agentdata-cancel')
    for (const [ch, ev, arg] of [
      ['ccm:agentdata-state', 'guest', undefined],
      ['ccm:agentdata-scan', 'guest', undefined],
      ['ccm:agentdata-cancel', 'guest', undefined],
      ['ccm:agentdata-clean', 'app', { agent: 'claude', groups: ['transcripts'], olderDays: 0 }],
      ['ccm:agentdata-clean', 'app', { agent: 'claude', groups: ['debugLogs'], olderDays: 5 }],
      ['ccm:agentdata-clean', 'app', ['claude']],
      ['ccm:agentdata-clean', 'guest', { agent: 'claude', groups: ['debugLogs'], olderDays: 0 }],
      ['ccm:agentdata-strip', 'app', { minDays: 10 }],
      ['ccm:agentdata-strip', 'app', { minDays: '30' }],
      ['ccm:agentdata-move', 'app', { agent: 'gemini' }],
      ['ccm:agentdata-undo', 'app', { agent: 'claude' }],
      ['ccm:agentdata-delete-original', 'app', { agent: 'codex' }],
      ['ccm:agentdata-discard', 'app', { agent: 'codex' }],
      ['ccm:agentdata-discard', 'guest', { agent: 'codex' }],
    ] as const) {
      same(await node.ipc.call(ch, ev, arg), await rust.ipc.call(ch, ev, arg))
    }
  })

  it('scan reports the same groups, entries and sizes', async () => {
    const scan = (await twinCall('ccm:agentdata-scan')) as Array<{ groups: Array<{ id: string }> }>
    expect(scan.map((s) => s.groups.map((g) => g.id))[0]).toContain('transcripts')
    const last = twin.events.filter((e) => e.event === 'ccm:agentdata-event').at(-1)?.payload
    expect(last).toEqual(node.events.at(-1))
  })

  it('clean refuses while the agent runs, honours cancel, then deletes the same files', async () => {
    const req = { agent: 'claude', groups: ['debugLogs', 'shellSnapshots', 'appCache'], olderDays: 30 }
    writeFileSync(procsFile, JSON.stringify([{ pid: 4242, name: '/usr/local/bin/claude' }, { pid: 4343, name: 'codex' }]))
    const running = (await twinCall('ccm:agentdata-clean', req)) as { error: string }
    expect(running.error).toBe('running')
    writeFileSync(procsFile, '[]')
    for (const s of [node, rust]) s.answer = 1
    expect(await twinCall('ccm:agentdata-clean', req)).toMatchObject({ error: 'cancelled' })
    for (const s of [node, rust]) s.answer = 0
    expect(await twinCall('ccm:agentdata-clean', req)).toMatchObject({ ok: true, files: 4 })
    same(node.confirms, rust.confirms)
    same(tree(node, 'claude'), tree(rust, 'claude'))
    expect(existsSync(path.join(node.claude, 'debug'))).toBe(true)
    expect(existsSync(path.join(node.claude, 'debug', 'deep'))).toBe(false)
    expect(await twinCall('ccm:agentdata-clean', req)).toMatchObject({ error: 'nothing' })
  })

  it('strip rewrites the same transcripts, skips the live session and backs up the originals', async () => {
    const r = (await twinCall('ccm:agentdata-strip', { minDays: 30 })) as { ok: boolean; files: number; path: string }
    expect(r).toMatchObject({ ok: true, files: 1, skipped: 1 })
    same(tree(node, 'claude'), tree(rust, 'claude'))
    same(tree(node, 'backup'), tree(rust, 'backup'))
    same(node.confirms.at(-1), rust.confirms.at(-1))
    expect(readdirSync(path.join(node.root, 'backup'))).toHaveLength(1)
    expect(await twinCall('ccm:agentdata-strip', { minDays: 30 })).toMatchObject({ ok: false, error: 'failed', files: 0, skipped: 1 })
  })

  it('move, undo with copy back, delete original and undo again match', async () => {
    const moved = await twinCall('ccm:agentdata-move', { agent: 'codex' })
    expect(moved).toMatchObject({ ok: true })
    expect(lstatSync(rust.codex).isSymbolicLink()).toBe(true)
    await twinCall('ccm:agentdata-state')
    same(tree(node, 'target'), tree(rust, 'target'))
    expect(await twinCall('ccm:agentdata-move', { agent: 'codex' })).toMatchObject({ error: 'already-moved' })
    const later = new Date(Date.now() + 60_000)
    for (const s of [node, rust]) {
      const f = path.join(s.root, 'target', 'codex', 'sessions', 'new.jsonl')
      put(f, '{"new":1}\n')
      utimesSync(f, later, later)
    }
    expect(await twinCall('ccm:agentdata-undo', { agent: 'codex' })).toMatchObject({ ok: true, files: 1 })
    expect(lstatSync(rust.codex).isDirectory()).toBe(true)
    same(tree(node, 'codex'), tree(rust, 'codex'))
    for (const s of [node, rust]) rmSync(path.join(s.root, 'target', 'codex'), { recursive: true, force: true })
    expect(await twinCall('ccm:agentdata-move', { agent: 'codex' })).toMatchObject({ ok: true })
    expect(await twinCall('ccm:agentdata-delete-original', { agent: 'codex' })).toMatchObject({ ok: true })
    await twinCall('ccm:agentdata-state')
    expect(await twinCall('ccm:agentdata-undo', { agent: 'codex' })).toMatchObject({ ok: true })
    same(tree(node, 'codex'), tree(rust, 'codex'))
    same(node.confirms, rust.confirms)
    expect(rust.collector).toEqual(node.collector)
    expect(rust.collector.length).toBeGreaterThan(0)
    expect(twin.fellBack).toEqual([])
    expect(existsSync(path.join(rust.root, 'outside', 'keep.txt'))).toBe(true)
    await twinCall('ccm:agentdata-state')
  }, 60_000)
})
