import { execFileSync } from 'node:child_process'
import { appendFile, copyFile, mkdir, mkdtemp, readdir, readFile, rm, truncate, utimes, writeFile } from 'node:fs/promises'
import os from 'node:os'
import path from 'node:path'
import { afterAll, afterEach, beforeAll, describe, expect, it } from 'vitest'
import type { MonitorEvent } from '@ccm/shared'
import type { CollectorConfig } from '../src/config'
import { CoreSideFiles } from '../src/coreSideFiles'
import { onCoreFallback, resetCoreFallback, type CoreFallbackMessage } from '../src/coreFallback'
import { createCoreTranscript, type CoreTranscriptClient } from '../src/coreTranscript'
import { DeepseekMonitor } from '../src/deepseek/monitor'
import { Monitor } from '../src/monitor'
import { NodeOutboxTail, nodeSideFiles } from '../src/nodeSideFiles'
import { createSideFileSources, sideFilesMode, type DirReader, type SideFileSources } from '../src/sideFileSource'
import { MonitorStore } from '../src/store'
import { bodyFixtures, agedFixtures, DATE_STRINGS, outboxCorpus, rawFixtures, SID, SID2, type SideFixture } from './sidefiles-parity-corpus'
import { BIN } from './tracker-twin'

const describeCore = BIN ? describe : describe.skip
const plain = (v: unknown) => JSON.parse(JSON.stringify(v)) as unknown
const sorted = (v: unknown[]) => [...v].sort((a, b) => JSON.stringify(a).localeCompare(JSON.stringify(b)))
const tally = { dirReads: 0, records: 0, outboxReads: 0, events: 0, dates: 0, scenarios: 0 }
const dirs: string[] = []
const temp = async (p: string) => {
  const d = await mkdtemp(path.join(os.tmpdir(), p))
  dirs.push(d)
  return d
}
const BRIDGES = path.resolve(__dirname, '../../../scripts')

let client: CoreTranscriptClient | null = null
const rust = () => new CoreSideFiles({ client: () => client, log: () => undefined })

afterEach(() => {
  onCoreFallback(null)
  resetCoreFallback()
})

async function populate(dir: string, fixtures: SideFixture[], now: number): Promise<void> {
  await mkdir(dir, { recursive: true })
  for (const f of fixtures) {
    const file = path.join(dir, f.name)
    await writeFile(file, f.content)
    if (f.ageMs !== undefined) await utimes(file, (now - f.ageMs) / 1000, (now - f.ageMs) / 1000)
  }
}

async function twinDirs(kind: 'hooks' | 'statusline', fixtures: SideFixture[], now = Date.now()): Promise<unknown[]> {
  const a = await temp(`ccm-sf-${kind}-node-`)
  const b = await temp(`ccm-sf-${kind}-rust-`)
  await populate(a, fixtures, now)
  await populate(b, fixtures, now)
  const node = kind === 'hooks' ? nodeSideFiles.hooks(a) : nodeSideFiles.statusLine(a)
  const core = rust()
  const rs = kind === 'hooks' ? core.hooks(b) : core.statusLine(b)
  const want = sorted(plain(await node.read(now)) as unknown[])
  const got = sorted(plain(await rs.read(now)) as unknown[])
  expect(got).toEqual(want)
  expect(sorted(await readdir(b))).toEqual(sorted(await readdir(a)))
  expect(core.fellBack).toBe(false)
  tally.dirReads++
  tally.records += got.length
  return got
}

describe('side files source', () => {
  it('reads in Node unless CCM_CORE_SIDEFILES=rust and a core binary is set', () => {
    expect(sideFilesMode({})).toBe('node')
    expect(sideFilesMode({ CCM_CORE_SIDEFILES: ' Rust ' })).toBe('rust')
    const got: CoreFallbackMessage[] = []
    onCoreFallback((m) => got.push(m))
    expect(createSideFileSources({})).toBe(nodeSideFiles)
    expect(createSideFileSources({ CCM_CORE_BIN: 'x' })).toBe(nodeSideFiles)
    expect(createSideFileSources({ CCM_CORE_SIDEFILES: 'rust' })).toBe(nodeSideFiles)
    expect(createSideFileSources({ CCM_CORE_SIDEFILES: 'rust', CCM_CORE_BIN: 'x' })).toBeInstanceOf(CoreSideFiles)
    expect(got.map((m) => m.kind)).toEqual(['sidefiles'])
  })

  it('falls back to Node with one notification when the core gives no answer', async () => {
    const dir = await temp('ccm-sf-fb-')
    await populate(path.join(dir, 'h'), bodyFixtures('hooks'), Date.now())
    await populate(path.join(dir, 's'), bodyFixtures('statusline'), Date.now())
    const outbox = path.join(dir, 'outbox.jsonl')
    await writeFile(outbox, outboxCorpus(3, 200))
    const got: CoreFallbackMessage[] = []
    onCoreFallback((m) => got.push(m))
    const dead: CoreTranscriptClient = { request: async () => null, send: () => false, dispose: () => undefined }
    const core = new CoreSideFiles({ client: () => dead, log: () => undefined })
    const now = Date.now()
    expect(plain(await core.hooks(path.join(dir, 'h')).read(now))).toEqual(plain(await nodeSideFiles.hooks(path.join(dir, 'h')).read(now)))
    expect(plain(await core.statusLine(path.join(dir, 's')).read(now))).toEqual(plain(await nodeSideFiles.statusLine(path.join(dir, 's')).read(now)))
    const seen: unknown[] = []
    const tail = core.outbox(outbox, { fromEnd: 50_000, maxLine: 256 * 1024 })
    await tail.readEach((e) => seen.push(e))
    const want: unknown[] = []
    await new NodeOutboxTail(outbox, { fromEnd: 50_000, maxLine: 256 * 1024 }).readEach((e) => want.push(e))
    expect(plain(seen)).toEqual(plain(want))
    expect(seen.length).toBeGreaterThan(0)
    expect(got.map((m) => m.kind)).toEqual(['sidefiles'])
    expect(core.fellBack).toBe(true)
  })
})

describeCore('side files: Node ↔ Rust parity', () => {
  beforeAll(() => {
    client = createCoreTranscript({ bin: BIN!, timeoutMs: 60_000 })
  })

  afterAll(async () => {
    client?.dispose()
    console.log(`[sidefiles parity] ${tally.dirReads} directory reads (${tally.records} records), ${tally.outboxReads} outbox reads (${tally.events} events), ${tally.dates} date strings, ${tally.scenarios} monitor scenarios compared`)
    for (const d of dirs) await rm(d, { recursive: true, force: true })
  })

  it('reads what hook-bridge.mjs and statusline-bridge.mjs write', async () => {
    const data = await temp('ccm-sf-bridge-')
    const env = { ...process.env, CCM_DATA_DIR: data }
    const hook = (payload: object) => execFileSync(process.execPath, [path.join(BRIDGES, 'hook-bridge.mjs')], { input: JSON.stringify(payload), env })
    const status = (payload: object) => execFileSync(process.execPath, [path.join(BRIDGES, 'statusline-bridge.mjs')], { input: JSON.stringify(payload), env })
    hook({ session_id: SID, hook_event_name: 'PreCompact', trigger: 'manual', custom_instructions: 'secret plan' })
    hook({ session_id: SID, hook_event_name: 'PermissionDenied', tool_name: 'Bash', tool_input: { command: 'rm -rf x' } })
    hook({ session_id: SID2, hook_event_name: 'PreCompact' })
    hook({ session_id: 'not-a-uuid', hook_event_name: 'PreCompact' })
    status({ session_id: SID, cost: { total_cost_usd: 2.5 }, context_window: { context_window_size: 1_000_000, used_percentage: 41.5 }, rate_limits: { five_hour: { used_percentage: 33, resets_at: 1_830_000_000 }, seven_day: { used_percentage: 120 } } })
    status({ session_id: SID2, context_window: { remaining_percentage: 80 } })
    status({ session_id: 'nope' })
    for (const [kind, sub] of [['hooks', 'hooks'], ['statusline', 'statusline']] as const) {
      const a = path.join(data, sub)
      const b = await temp(`ccm-sf-bridge-${kind}-`)
      for (const name of await readdir(a)) await copyFile(path.join(a, name), path.join(b, name))
      const now = Date.now()
      const node = kind === 'hooks' ? nodeSideFiles.hooks(a) : nodeSideFiles.statusLine(a)
      const core = rust()
      const rs = kind === 'hooks' ? core.hooks(b) : core.statusLine(b)
      const want = plain(await node.read(now)) as unknown[]
      expect(want.length).toBeGreaterThanOrEqual(2)
      expect(sorted(plain(await rs.read(now)) as unknown[])).toEqual(sorted(want))
      tally.dirReads++
      tally.records += want.length
    }
    expect(JSON.stringify(await readFile(path.join(data, 'hooks', `${SID}.json`), 'utf8'))).not.toMatch(/secret|rm -rf/)
  })

  it.each(['hooks', 'statusline'] as const)('%s: odd bodies give the same records', async (kind) => {
    const got = await twinDirs(kind, bodyFixtures(kind))
    expect(got.length).toBeGreaterThan(5)
  })

  it.each(['hooks', 'statusline'] as const)('%s: broken JSON, oversize, wrong names and encodings give the same records', async (kind) => {
    const got = await twinDirs(kind, rawFixtures())
    expect(got.length).toBeGreaterThan(2)
  })

  it.each([
    ['hooks', 24],
    ['statusline', 24 * 7],
  ] as const)('%s: sweeps only valid-named files older than KEEP_MS, only in the configured directory', async (kind, keepHours) => {
    const now = Date.now()
    const fixtures = agedFixtures()
    const got = await twinDirs(kind, fixtures, now)
    const keepMs = keepHours * 3_600_000
    const expected = fixtures.filter((f) => /^[0-9a-f-]{36}\.json$/.test(f.name) && f.content !== '{"broken":' && (f.ageMs ?? 0) <= keepMs).length
    expect(got).toHaveLength(expected)
    const bystander = await temp('ccm-sf-bystander-')
    await populate(bystander, fixtures, now)
    const target = await temp('ccm-sf-target-')
    await populate(target, fixtures, now)
    const core = rust()
    await (kind === 'hooks' ? core.hooks(target) : core.statusLine(target)).read(now)
    expect(sorted(await readdir(bystander))).toEqual(sorted(fixtures.map((f) => f.name)))
    const left = await readdir(target)
    expect(left).toContain('notes.json')
    expect(left.some((n) => n.endsWith('.json.tmp'))).toBe(true)
    expect(left.length).toBeLessThan(fixtures.length)
  })

  it('stays the same when files vanish, change or come back between reads', async () => {
    const a = await temp('ccm-sf-vanish-node-')
    const b = await temp('ccm-sf-vanish-rust-')
    const fixtures = bodyFixtures('statusline').slice(0, 8)
    await populate(a, fixtures, Date.now())
    await populate(b, fixtures, Date.now())
    const node = nodeSideFiles.statusLine(a)
    const core = rust()
    const rs = core.statusLine(b)
    const same = async () => {
      const now = Date.now()
      expect(sorted(plain(await rs.read(now)) as unknown[])).toEqual(sorted(plain(await node.read(now)) as unknown[]))
      tally.dirReads++
    }
    await same()
    for (const d of [a, b]) await rm(path.join(d, fixtures[1]!.name))
    await same()
    for (const d of [a, b]) await writeFile(path.join(d, fixtures[2]!.name), JSON.stringify({ sessionId: SID, at: '2026-11-11T00:00:00Z', costUsd: 9 }))
    await same()
    for (const d of [a, b]) await writeFile(path.join(d, fixtures[1]!.name), fixtures[1]!.content)
    await same()
    await rm(a, { recursive: true })
    await rm(b, { recursive: true })
    await same()
    expect(core.fellBack).toBe(false)
  })

  it('accepts the same date strings as Date.parse, apart from the known legacy formats', async () => {
    const stamp = (s: string, i: number) => ({ name: `${SID.slice(0, 8 - String(i).length)}${i}${SID.slice(8)}.json`, content: JSON.stringify({ sessionId: SID, at: s }) })
    const fixtures = DATE_STRINGS.map(stamp)
    const a = await temp('ccm-sf-date-node-')
    const b = await temp('ccm-sf-date-rust-')
    await populate(a, fixtures, Date.now())
    await populate(b, fixtures, Date.now())
    const now = Date.now()
    const accepted = (rows: unknown[]) => new Set((rows as Array<{ at: string }>).map((r) => r.at))
    const node = accepted(await nodeSideFiles.statusLine(a).read(now))
    const rs = accepted(await rust().statusLine(b).read(now))
    const differ = DATE_STRINGS.filter((s) => node.has(s) !== rs.has(s))
    tally.dates += DATE_STRINGS.length
    for (const s of DATE_STRINGS) expect(node.has(s)).toBe(Number.isFinite(Date.parse(s)))
    expect(differ).toEqual(KNOWN_DATE_DIFFS)
    for (const s of differ) expect(rs.has(s)).toBe(false)
  })

  const outboxTwin = (file: string, opts: { fromEnd?: number; maxLine?: number } = {}) => {
    const node = new NodeOutboxTail(file, opts)
    const core = rust()
    const rs = core.outbox(file, opts)
    return async () => {
      const a: unknown[] = []
      const b: unknown[] = []
      await node.readEach((e) => a.push(e))
      await rs.readEach((e) => b.push(e))
      expect(plain(b)).toEqual(plain(a))
      expect(core.fellBack).toBe(false)
      tally.outboxReads++
      tally.events += b.length
      return b
    }
  }

  it('outbox: odd lines, long lines and ids give the same events', async () => {
    const dir = await temp('ccm-sf-outbox-')
    const file = path.join(dir, 'outbox.jsonl')
    await writeFile(file, outboxCorpus(11, 1500))
    const events = await outboxTwin(file, { maxLine: 256 * 1024 })()
    expect(events.length).toBeGreaterThan(300)
    expect(JSON.stringify(events)).not.toContain('free text')
    await writeFile(file, outboxCorpus(12, 400))
    await outboxTwin(file, { maxLine: 200_000 })()
  })

  it('outbox: appended in random pieces, truncated, resumed from the end and rotated', async () => {
    const dir = await temp('ccm-sf-outbox-pieces-')
    const file = path.join(dir, 'outbox.jsonl')
    for (const seed of [1, 7, 42]) {
      const text = outboxCorpus(seed, 3000)
      expect(text.length).toBeGreaterThan(1_500_000)
      await writeFile(file, '')
      const whole = outboxTwin(file, { maxLine: 256 * 1024 })
      let at = 0
      let step = 0
      while (at < text.length) {
        const n = Math.max(1, Math.floor((((seed * 7919 + step * 104729) % 1000) / 1000) * 700_000))
        await appendFile(file, text.subarray(at, at + n))
        at += n
        step++
        await whole()
      }
      const resumed = outboxTwin(file, { fromEnd: 300_000 + seed * 111, maxLine: 256 * 1024 })
      await resumed()
      await truncate(file, Math.floor(text.length / 3))
      await appendFile(file, text.subarray(0, 100_000))
      await whole()
      await resumed()
      await appendFile(file, `${JSON.stringify({ event: 'turn_start', thread_id: 'tail-1', timestamp: 1_730_000_000 })}\n{"event":"turn_end","thr`)
      await whole()
      await appendFile(file, 'ead_id":"tail-1","kind":"turn.failed"}\n')
      const last = (await resumed()) as Array<{ threadId: string; kind?: string }>
      expect(last.map((e) => e.threadId)).toEqual(['tail-1'])
      expect(last[0]?.kind).toBe('turn.failed')
      await rm(file)
      await whole()
      await writeFile(file, text.subarray(0, 50_000))
      await outboxTwin(file, { fromEnd: 20_000 })()
    }
  }, 120_000)

  it('outbox: a missing file reads nothing and a core that stops mid-way hands over to Node without loss or repeat', async () => {
    const dir = await temp('ccm-sf-outbox-mid-')
    const file = path.join(dir, 'outbox.jsonl')
    expect(await outboxTwin(path.join(dir, 'none.jsonl'))()).toEqual([])
    const text = outboxCorpus(5, 2500)
    await writeFile(file, text.subarray(0, text.length >> 1))
    let left = 3
    const flaky: CoreTranscriptClient = { request: (op, extra, o) => (left-- > 0 ? client!.request(op, extra, o) : Promise.resolve(null)), send: () => true, dispose: () => undefined }
    const got: CoreFallbackMessage[] = []
    onCoreFallback((m) => got.push(m))
    const core = new CoreSideFiles({ client: () => flaky, log: () => undefined })
    const rs = core.outbox(file, { maxLine: 256 * 1024 })
    const node = new NodeOutboxTail(file, { maxLine: 256 * 1024 })
    const a: unknown[] = []
    const b: unknown[] = []
    for (const upto of [text.length >> 1, text.length]) {
      await appendFile(file, text.subarray((await readFile(file)).length, upto))
      await node.readEach((e) => a.push(e))
      await rs.readEach((e) => b.push(e))
    }
    expect(plain(b)).toEqual(plain(a))
    expect(a.length).toBeGreaterThan(300)
    expect(core.fellBack).toBe(true)
    expect(got.map((m) => m.kind)).toEqual(['sidefiles'])
  })

  it('Monitor.pollExtras publishes the same limits through both paths', async () => {
    const root = await temp('ccm-sf-monitor-')
    await populate(path.join(root, 'statusline'), bodyFixtures('statusline'), Date.now())
    await populate(path.join(root, 'hooks'), bodyFixtures('hooks'), Date.now())
    const config = (): CollectorConfig => ({
      claudeRoot: root, codexHome: root, codex: false, codewhaleHome: root, deepseek: false, host: '127.0.0.1', port: 0, toast: false, toastBackend: null, appKey: null, dev: true, toastLink: null,
      staleTtlMs: 120 * 60_000, reconcileMs: 5_000, processVerifyMs: 60_000, activityLimit: 300, batchMs: 1, verifyProcesses: false, webDist: root, dataDir: root, devOriginPorts: [],
      statusLineBridge: path.join(root, 'statusline-bridge.mjs'), statusLineCommand: 'node statusline-bridge.mjs',
    })
    const limits = async (sideFiles: SideFileSources) => {
      const store = new MonitorStore({ activityLimit: 100, batchMs: 1 })
      const m = new Monitor(config(), store, { isPidAlive: () => true, queryProcesses: async () => null, now: () => Date.now(), sideFiles })
      await m.pollExtras()
      return plain((store.snapshotMessage().events[0] as Extract<MonitorEvent, { type: 'snapshot' }>).payload.limits)
    }
    const want = await limits(nodeSideFiles)
    expect(want).toBeTruthy()
    expect(await limits(rust())).toEqual(want)
    tally.scenarios++
  })

  it('DeepseekMonitor follows the same session through both paths', async () => {
    const T0 = Date.parse('2026-11-03T10:00:00Z')
    const at = (s: number) => new Date(T0 + s * 1000).toISOString()
    const line = (event: string, kind: string, s: number, payload: Record<string, unknown> = {}) => `${JSON.stringify({ event, kind, thread_id: 'th-1', timestamp: at(s), payload: { workspace: '/home/cw/wt-lane', ...payload } })}\n`
    const run = async (sideFiles: SideFileSources) => {
      const home = await temp('ccm-sf-ds-')
      const outbox = path.join(home, 'notifications', 'outbox.jsonl')
      await mkdir(path.dirname(outbox), { recursive: true })
      await writeFile(path.join(home, 'config.toml'), '[lifecycle_outbox]\npath = "~/notifications/outbox.jsonl"\n')
      await writeFile(outbox, line('session_start', 'session.started', 0) + line('turn_start', 'turn.started', 1))
      const store = new MonitorStore({ activityLimit: 100, batchMs: 1 })
      const ds = new DeepseekMonitor(
        { codewhaleHome: home, homeDir: home, staleTtlMs: 120 * 60_000, reconcileMs: 5_000, verifyMs: 60_000, verifyProcesses: true },
        store,
        { listProcesses: async () => [{ pid: 500, start: T0 - 500, ancestors: [400] }], isPidAlive: () => true, now: () => T0 + 10_000, sideFiles },
      )
      await ds.start({ watch: false })
      const read = () => (ds as unknown as { readOutbox(c: boolean): Promise<void> }).readOutbox(false)
      const views: unknown[] = []
      const view = () => {
        const s = store.snapshot()
        views.push(plain({ terminals: s.terminals, agents: s.agents, activity: s.activity.map(({ id: _id, ...a }) => (a.kind === 'terminal.ended' ? { ...a, at: undefined } : a)) }))
      }
      view()
      await appendFile(outbox, line('subagent_spawn', 'subagent.spawned', 2, { subagent: { id: 'sa', name: 'explorer' } }) + line('turn_end', 'turn.failed', 3) + 'garbage\n')
      await read()
      view()
      await appendFile(outbox, line('subagent_complete', 'subagent.completed', 4, { subagent: { id: 'sa' }, status: 'Done' }) + '{"event":"turn_start","thr')
      await read()
      view()
      await appendFile(outbox, 'ead_id":"th-1","timestamp":1730000000}\n' + line('session_end', 'session.ended', 5))
      await read()
      view()
      ds.stop()
      return views
    }
    const want = await run(nodeSideFiles)
    const core = rust()
    expect(await run(core)).toEqual(want)
    expect(core.fellBack).toBe(false)
    expect(JSON.stringify(want)).toContain('explorer')
    tally.scenarios++
  })
})

const KNOWN_DATE_DIFFS = [
  '2026-10-29T01:00:00-0700',
  '2026-10-29T01:00:00+0700',
  '2026-10-29T01:00:00z',
  '2026-10-29t01:00:00Z',
  '2026-1-1',
  '+002026-10-29T00:00:00Z',
  '-000001-01-01T00:00:00Z',
  'Oct 29 2026',
  'Thu, 29 Oct 2026 01:00:00 GMT',
  '10/29/2026',
  '2026/10/29',
]
