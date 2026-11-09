import { spawn } from 'node:child_process'
import { existsSync } from 'node:fs'
import { appendFile, chmod, copyFile, mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import os from 'node:os'
import path from 'node:path'
import { afterAll, describe, expect, it } from 'vitest'
import type { UsageBucket, UsageScan, UsageSnapshot } from '@ccm/shared'
import { createCoreTranscript } from '../src/coreTranscript'
import { localDay, UsageIndex, type UsageSink } from '../src/usage'
import { createUsageSource, RustUsageSource, usageMode, type UsageSource } from '../src/usageSource'

const exe = process.platform === 'win32' ? 'orrery-core.exe' : 'orrery-core'
const coreDir = path.resolve(__dirname, '../../core/target')
const BIN = process.env.CCM_CORE_BIN?.trim() || [path.join(coreDir, 'debug', exe), path.join(coreDir, 'release', exe)].find((p) => existsSync(p))

const line = (o: unknown) => `${JSON.stringify(o)}\n`
const TS = '2026-10-07T09:00:00.000Z'
const DAY = localDay(TS)!
const OPTS = { sweepMs: 0, fileDebounceMs: 1, saveDebounceMs: 1 }

class Sink implements UsageSink {
  snapshot: UsageSnapshot = { buckets: [], scan: { state: 'idle', filesDone: 0, filesTotal: 0 } }
  scans: UsageScan[] = []
  resetUsage(s: UsageSnapshot) {
    this.snapshot = { buckets: s.buckets.map((b) => ({ ...b })), scan: { ...s.scan } }
  }
  updateUsage(b: UsageBucket[]) {
    for (const x of b) {
      const i = this.snapshot.buckets.findIndex((y) => y.key === x.key)
      if (i >= 0) this.snapshot.buckets[i] = { ...x }
      else this.snapshot.buckets.push({ ...x })
    }
  }
  setUsageScan(s: UsageScan) {
    this.scans.push(s)
  }
  sorted() {
    return [...this.snapshot.buckets].sort((a, b) => (a.key < b.key ? -1 : a.key > b.key ? 1 : 0))
  }
}

const msg = (o: { id: string; model?: string; ts?: string; cwd?: string | null; usage?: Record<string, unknown>; content?: unknown[] }) =>
  line({
    type: 'assistant',
    timestamp: o.ts ?? TS,
    ...(o.cwd === null ? {} : { cwd: o.cwd ?? 'D:\\work\\demo' }),
    message: { id: o.id, model: o.model ?? 'claude-opus-5-5', content: o.content ?? [{ type: 'text', text: 'secret' }], usage: o.usage ?? { input_tokens: 3, output_tokens: 5 } },
  })
const use = (id: string, tid: string, name: string, input: Record<string, unknown> = {}, cwd?: string) => msg({ id, cwd, usage: { output_tokens: 1 }, content: [{ type: 'tool_use', id: tid, name, input }] })
const result = (tid: string, isError: boolean) => line({ type: 'user', timestamp: TS, message: { role: 'user', content: [{ type: 'tool_result', tool_use_id: tid, is_error: isError, content: 'boom' }] } })

async function corpus(projects: string): Promise<void> {
  const demo = path.join(projects, 'D--work-demo')
  await mkdir(path.join(demo, 'sid', 'subagents'), { recursive: true })
  await mkdir(path.join(projects, 'E--other'), { recursive: true })
  await writeFile(
    path.join(demo, 'sid.jsonl'),
    msg({ id: 'm1', usage: { input_tokens: 5, output_tokens: 7, cache_creation_input_tokens: 30, cache_creation: { ephemeral_5m_input_tokens: 10, ephemeral_1h_input_tokens: 20 } } }) +
      msg({ id: 'm1', usage: { input_tokens: 5, output_tokens: 7 } }) +
      msg({ id: 'm2', ts: '2026-10-05T23:30:00.000Z', usage: { cache_creation_input_tokens: 9, speed: 'fast', cache_read_input_tokens: 100 } }) +
      msg({ id: 'm3', model: '<synthetic>' }) +
      use('m4', 'tu1', 'Bash', { command: 'rm -rf secret' }) +
      use('m4', 'tu2', 'Skill', { skill: 'pdf' }) +
      result('tu1', true) +
      result('tu2', false) +
      use('m5', 'tu3', 'mcp__github__get_me') +
      result('tu3', true) +
      use('m6', 'tu4', 'Agent', { subagent_type: 'Explore', prompt: 'secret' }) +
      use('m7', 'tu5', 'Task', {}) +
      use('m8', 'tu6', 'Edit') +
      use('m9', 'tu7', 'Glob') +
      msg({ id: 'm10', ts: '2026-10-06T10:15:00+07:00' }).replace('\n', '\r\n') +
      '{"type":"assistant","usage":broken\n' +
      msg({ id: 'm11', usage: { input_tokens: 1 }, content: [{ type: 'text', text: 'x'.repeat(1_300_000) }] }) +
      msg({ id: 'm12', usage: { output_tokens: 40 } }).slice(0, 30),
  )
  await writeFile(path.join(demo, 'sid', 'subagents', 'agent-a.jsonl'), msg({ id: 's1', model: 'claude-haiku-4-5', usage: { output_tokens: 3 } }) + use('s2', 'tu8', 'Read', {}, 'E:\\other'))
  await writeFile(path.join(demo, 'resumed.jsonl'), msg({ id: 'm1', usage: { input_tokens: 5, output_tokens: 7 } }) + use('m4', 'tu1', 'Bash') + msg({ id: 'r1', usage: { output_tokens: 2 } }))
  await writeFile(path.join(projects, 'E--other', 'x.jsonl'), msg({ id: 'o1', cwd: null, model: 'claude-sonnet-5', ts: '2026-09-20T12:00:00.000Z' }) + use('o2', 'tu9', 'mcp__claude_ai_Gmail__send', {}, 'E:\\other') + result('tu9', true))
}

const roots: string[] = []
afterAll(async () => {
  for (const r of roots) await rm(r, { recursive: true, force: true })
})

async function tmpRoot(): Promise<string> {
  const root = await mkdtemp(path.join(os.tmpdir(), 'ccm-usage-parity-'))
  roots.push(root)
  return root
}

const rust = (projects: string, cache: string, sink: UsageSink) => new RustUsageSource(projects, cache, sink, OPTS, (hooks) => createCoreTranscript({ bin: BIN!, ...hooks }), (l) => {
  throw new Error(l)
})

async function started(src: UsageSource): Promise<UsageSource> {
  await src.start()
  await src.whenIdle()
  return src
}

async function views(src: UsageSource, sink: Sink) {
  return {
    buckets: sink.sorted(),
    scan: src.scanInfo(),
    heat: await src.heatmap(28, DAY),
    all: await src.toolStats(30, null, DAY),
    other: await src.toolStats(7, 'E:\\other', DAY),
  }
}

const cacheOf = async (file: string) => JSON.parse(await readFile(file, 'utf8')) as unknown

const settle = async (src: UsageSource) => {
  await new Promise((r) => setTimeout(r, 30))
  await src.whenIdle()
}

describe.skipIf(!BIN)('usage index parity Node ↔ Rust', () => {
  it('gives the same buckets, queries and cache, also after appends, and each reads the other cache', async () => {
    const root = await tmpRoot()
    const projects = path.join(root, 'projects')
    await corpus(projects)
    const nodeSink = new Sink()
    const rustSink = new Sink()
    const node = await started(new UsageIndex(projects, path.join(root, 'node.json'), nodeSink, OPTS))
    const core = await started(rust(projects, path.join(root, 'rust.json'), rustSink))
    const first = await views(node, nodeSink)
    expect(first.buckets.length).toBeGreaterThan(4)
    expect(first.all.tools.length).toBeGreaterThan(4)
    expect((core as RustUsageSource).usingNode).toBe(false)
    expect(await views(core, rustSink)).toEqual(first)
    expect(JSON.stringify(rustSink.snapshot)).not.toContain('secret')

    const demo = path.join(projects, 'D--work-demo')
    const tail = msg({ id: 'm12', usage: { output_tokens: 40 } })
    await appendFile(path.join(demo, 'sid.jsonl'), tail.slice(30) + use('m13', 'tu10', 'Bash') + result('tu10', true))
    await writeFile(path.join(demo, 'resumed.jsonl'), msg({ id: 'n1', usage: { output_tokens: 4 } }))
    for (const src of [node, core]) {
      src.notify(path.join('D--work-demo', 'sid.jsonl'))
      src.notify(path.join('D--work-demo', 'resumed.jsonl'))
      await settle(src)
    }
    const second = await views(node, nodeSink)
    expect(second).not.toEqual(first)
    expect(await views(core, rustSink)).toEqual(second)
    await node.stop()
    await core.stop()
    expect(await cacheOf(path.join(root, 'rust.json'))).toEqual(await cacheOf(path.join(root, 'node.json')))

    await copyFile(path.join(root, 'rust.json'), path.join(root, 'from-rust.json'))
    await copyFile(path.join(root, 'node.json'), path.join(root, 'from-node.json'))
    const nodeAgainSink = new Sink()
    const rustAgainSink = new Sink()
    const nodeAgain = await started(new UsageIndex(projects, path.join(root, 'from-rust.json'), nodeAgainSink, OPTS))
    const rustAgain = await started(rust(projects, path.join(root, 'from-node.json'), rustAgainSink))
    expect(await views(nodeAgain, nodeAgainSink)).toEqual(second)
    expect((rustAgain as RustUsageSource).usingNode).toBe(false)
    expect(await views(rustAgain, rustAgainSink)).toEqual(second)
    await rm(path.join(demo, 'resumed.jsonl'))
    for (const src of [nodeAgain, rustAgain]) await src.stop()
    expect(await cacheOf(path.join(root, 'from-node.json'))).toEqual(await cacheOf(path.join(root, 'from-rust.json')))
  }, 60_000)
})

async function lockFile(file: string): Promise<() => Promise<void>> {
  if (process.platform !== 'win32') {
    await chmod(file, 0o000)
    return () => chmod(file, 0o644)
  }
  const quoted = file.replace(/'/g, "''")
  const script = `$f=[IO.File]::Open('${quoted}','Open','Read','None'); [Console]::Out.WriteLine('locked'); [void][Console]::In.ReadLine(); $f.Close()`
  const child = spawn('powershell', ['-NoProfile', '-NonInteractive', '-Command', script], { stdio: ['pipe', 'pipe', 'ignore'], windowsHide: true })
  await new Promise<void>((resolve, reject) => {
    child.stdout.on('data', (d: Buffer) => d.toString().includes('locked') && resolve())
    child.on('exit', () => reject(new Error('lock helper exited')))
  })
  return () =>
    new Promise<void>((resolve) => {
      child.on('exit', () => resolve())
      child.stdin.end('\n')
    })
}

describe.skipIf(!BIN)('usage index skips an unreadable transcript (Node and Rust)', () => {
  it('logs, keeps scanning the other files and counts the file once it can be read', async () => {
    const root = await tmpRoot()
    const projects = path.join(root, 'projects')
    await corpus(projects)
    const locked = path.join(projects, 'E--other', 'x.jsonl')
    const unlock = await lockFile(locked)
    const nodeSink = new Sink()
    const rustSink = new Sink()
    let node!: UsageSource
    let core!: UsageSource
    try {
      node = await started(new UsageIndex(projects, path.join(root, 'node.json'), nodeSink, OPTS))
      core = await started(rust(projects, path.join(root, 'rust.json'), rustSink))
    } finally {
      await unlock()
    }
    const before = await views(node, nodeSink)
    expect(before.scan.state).toBe('ready')
    expect(before.buckets.some((b) => b.model === 'claude-sonnet-5')).toBe(false)
    expect(before.buckets.some((b) => b.model === 'claude-opus-5-5')).toBe(true)
    expect(await views(core, rustSink)).toEqual(before)
    for (const src of [node, core]) {
      src.notify(path.join('E--other', 'x.jsonl'))
      await settle(src)
    }
    const after = await views(node, nodeSink)
    expect(after.buckets.some((b) => b.model === 'claude-sonnet-5')).toBe(true)
    expect(await views(core, rustSink)).toEqual(after)
    await node.stop()
    await core.stop()
    expect(await cacheOf(path.join(root, 'rust.json'))).toEqual(await cacheOf(path.join(root, 'node.json')))
  }, 60_000)
})

describe('usage source selection', () => {
  it('defaults to the Node index and falls back to it when the core cannot start', async () => {
    expect(usageMode({})).toBe('node')
    expect(usageMode({ CCM_CORE_USAGE: ' Rust ' })).toBe('rust')
    expect(createUsageSource('p', null, new Sink(), {}, {})).toBeInstanceOf(UsageIndex)
    expect(createUsageSource('p', null, new Sink(), {}, { CCM_CORE_USAGE: 'rust' })).toBeInstanceOf(UsageIndex)
    expect(createUsageSource('p', null, new Sink(), {}, { CCM_CORE_USAGE: 'rust', CCM_CORE_BIN: 'x' })).toBeInstanceOf(RustUsageSource)

    const root = await tmpRoot()
    const projects = path.join(root, 'projects')
    await corpus(projects)
    const nodeSink = new Sink()
    const node = await started(new UsageIndex(projects, path.join(root, 'n.json'), nodeSink, OPTS))
    const logs: string[] = []
    for (const make of [() => null, () => createCoreTranscript({ bin: path.join(root, 'missing-core.exe'), log: (l) => logs.push(l) })]) {
      const sink = new Sink()
      const src = new RustUsageSource(projects, path.join(root, `f${logs.length}.json`), sink, OPTS, make, (l) => logs.push(l))
      await started(src)
      expect(src.usingNode).toBe(true)
      expect(await views(src, sink)).toEqual(await views(node, nodeSink))
      await src.stop()
    }
    expect(logs.some((l) => l.includes('using the Node usage index'))).toBe(true)
    await node.stop()
  }, 30_000)
})
