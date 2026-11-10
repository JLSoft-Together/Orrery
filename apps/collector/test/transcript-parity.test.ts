import { spawn, execFileSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import { appendFile, mkdtemp, readdir, rm, stat, writeFile } from 'node:fs/promises'
import os from 'node:os'
import path from 'node:path'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import { createCoreTranscript, type CoreTranscriptClient } from '../src/coreTranscript'
import type { RegistryRecord } from '../src/registry'
import { JsonlTailer, type LineFilter } from '../src/tailer'
import { ToolSpanLog } from '../src/tool-spans'
import { SessionTracker, signalLineFilter } from '../src/tracker'
import { extractSignals, TranscriptState, type Signal } from '../src/transcript'
import { edgeLines, resumeCopy, trackerBackground, trackerMain, trackerMessages } from './transcript-parity-corpus'

const exe = process.platform === 'win32' ? 'orrery-core.exe' : 'orrery-core'
const coreDir = path.resolve(__dirname, '../../core/target')
const BIN = process.env.CCM_CORE_BIN?.trim() || [path.join(coreDir, 'debug', exe), path.join(coreDir, 'release', exe)].find((p) => existsSync(p))
const DEMO = path.resolve(__dirname, '../../../scripts/demo-fixture.mjs')
const NOW = Date.parse('2026-10-07T14:00:00.000Z')

const LONE_SURROGATE = /[\ud800-\udbff](?![\udc00-\udfff])|(?<![\ud800-\udbff])[\udc00-\udfff]/g
const json = <T>(v: T): T => JSON.parse(JSON.stringify(v), (_k, x: unknown) => (typeof x === 'string' ? x.replace(LONE_SURROGATE, '') : x)) as T

function stateJson(s: TranscriptState) {
  return json({
    openTools: [...s.openTools],
    codexCalls: s.codexCalls,
    deviceCallAt: s.deviceCallAt,
    portConflict: s.portConflict,
    model: s.model,
    effort: s.effort,
    cacheTtl: s.cacheTtl,
    cacheAt: s.cacheAt,
    lastAt: s.lastAt,
    contextTokens: s.contextTokens,
    firstContext: s.firstContext,
    peakContext: s.peakContext,
    compactions: s.compactions,
    permissionMode: s.permissionMode,
    gitBranch: s.gitBranch,
    ended: s.ended,
    lastEndAt: s.lastEndAt,
    limitHit: s.limitHit,
    lastReply: s.lastReply,
    tokens: s.tokens(),
    currentTool: s.currentTool(),
    currentToolSummary: s.currentToolSummary(),
    currentToolStartedAt: s.currentToolStartedAt(),
    pendingAsk: s.pendingAsk(),
  })
}

interface Opts {
  lite?: string[]
  spans?: { terminalId: string; agentId: string }
  chunkBytes?: number
  maxBytes?: number
}

interface ReadOut {
  signals: Signal[]
  state: unknown
  spans?: unknown[]
}

class TsReader {
  private tailer: JsonlTailer
  private state = new TranscriptState()
  private spans?: ToolSpanLog
  private filter: LineFilter
  private now = NOW

  constructor(
    file: string,
    private o: Opts,
  ) {
    this.tailer = new JsonlTailer(file, o.chunkBytes)
    if (o.spans) this.spans = new ToolSpanLog(o.spans.terminalId, () => this.now)
    if (o.lite) {
      const t = new SessionTracker('t', { pid: 1 } as RegistryRecord, path.dirname(file)) as unknown as { agentToolUses: Set<string>; liteFilter: LineFilter }
      t.agentToolUses = new Set(o.lite)
      this.filter = t.liteFilter
    } else this.filter = signalLineFilter
  }

  async read(now: number): Promise<ReadOut> {
    this.now = now
    const signals: Signal[] = []
    const spans: unknown[] = []
    await this.tailer.readEach((rec) => {
      const s = extractSignals(rec)
      signals.push(...s)
      if (this.spans) spans.push(...this.spans.apply(s, this.o.spans!.agentId))
      this.state.apply(s)
    }, this.filter)
    return json({ signals, state: stateJson(this.state), ...(this.spans ? { spans } : {}) })
  }
}

class RustReader {
  constructor(
    private core: CoreTranscriptClient,
    private handle: number,
    private maxBytes?: number,
  ) {}

  static async open(core: CoreTranscriptClient, file: string, o: Opts): Promise<RustReader> {
    const r = await core.request('signals.open', { file, lite: !!o.lite, toolUseIds: o.lite, spans: o.spans, chunkBytes: o.chunkBytes })
    expect(r?.handle).toBeTypeOf('number')
    return new RustReader(core, r!.handle as number, o.maxBytes)
  }

  async read(now: number): Promise<ReadOut> {
    const signals: Signal[] = []
    const spans: unknown[] = []
    for (;;) {
      const r = await this.core.request('signals.read', { handle: this.handle, nowMs: now, maxBytes: this.maxBytes }, { timeoutMs: 60_000 })
      expect(r).not.toBeNull()
      signals.push(...(r!.signals as Signal[]))
      if (r!.spans) spans.push(...(r!.spans as unknown[]))
      if (r!.more === true) {
        pages++
        continue
      }
      return { signals, state: r!.state, ...(r!.spans ? { spans } : {}) }
    }
  }

  close(): Promise<unknown> {
    return this.core.request('signals.close', { handle: this.handle })
  }
}

let client: CoreTranscriptClient | null = null
let pages = 0
const core = () => client!
const tally = { files: 0, reads: 0, lines: 0, signals: 0, kinds: new Set<string>() }

async function pair(file: string, o: Opts = {}) {
  const ts = new TsReader(file, o)
  const rust = await RustReader.open(core(), file, o)
  return {
    async step(now = NOW): Promise<ReadOut> {
      const [a, b] = await Promise.all([ts.read(now), rust.read(now)])
      expect(b).toEqual(a)
      tally.reads++
      tally.signals += a.signals.length
      for (const s of a.signals) tally.kinds.add(s.k)
      return a
    },
    close: () => rust.close(),
  }
}

async function countLines(file: string): Promise<number> {
  const { readFile } = await import('node:fs/promises')
  const text = await readFile(file, 'utf8')
  return text.split('\n').filter((l) => l.trim()).length
}

async function whole(file: string, o: Opts = {}): Promise<ReadOut> {
  const p = await pair(file, o)
  const out = await p.step()
  await p.close()
  tally.files++
  tally.lines += await countLines(file)
  return out
}

let dir = ''

const describeCore = BIN ? describe : describe.skip

describeCore('transcript signals: Node ↔ Rust parity', () => {
  beforeAll(async () => {
    dir = await mkdtemp(path.join(os.tmpdir(), 'ccm-t2-parity-'))
    client = createCoreTranscript({ bin: BIN!, timeoutMs: 60_000 })
  })

  afterAll(async () => {
    client?.dispose()
    console.log(`[parity] ${tally.files} files, ${tally.lines} lines, ${tally.reads} reads, ${tally.signals} signals, ${pages} extra pages, ${tally.kinds.size} kinds: ${[...tally.kinds].sort().join(' ')}`)
    await rm(dir, { recursive: true, force: true })
  })

  it('matches on the tracker-shaped transcripts, main and subagents, full and lite', async () => {
    const main = path.join(dir, 'main.jsonl')
    const bg = path.join(dir, 'agent-bg.jsonl')
    const msgs = path.join(dir, 'messages.jsonl')
    await writeFile(main, trackerMain())
    await writeFile(bg, trackerBackground())
    await writeFile(msgs, trackerMessages())
    const spans = { terminalId: '4242:1343584', agentId: 'main' }
    const m = await whole(main, { spans })
    expect(m.signals.length).toBeGreaterThan(30)
    await whole(bg, { spans: { ...spans, agentId: 'bg' } })
    const lite = await whole(bg, { lite: ['toolu_g3', 'toolu_nested'], maxBytes: 200_000 })
    expect(lite.signals.length).toBeLessThan((await whole(bg)).signals.length)
    const out = await whole(msgs)
    expect(out.signals.map((s) => s.k)).toEqual(expect.arrayContaining(['coordinator_msg', 'handback', 'peer_in', 'peer_sent', 'send_main']))
    const copy = path.join(dir, 'resumed.jsonl')
    await writeFile(copy, resumeCopy(trackerMain()) + trackerMain())
    await whole(copy, { spans })
  })

  it('matches line by line on edge cases, through partial lines, appends and truncation', async () => {
    const lines = edgeLines()
    for (const chunkBytes of [7, 64, undefined]) {
      const file = path.join(dir, `edge-${chunkBytes ?? 'default'}.jsonl`)
      await writeFile(file, '')
      const p = await pair(file, { chunkBytes, maxBytes: chunkBytes === 7 ? 50 : undefined, spans: { terminalId: 't', agentId: 'main' } })
      for (const l of lines) {
        const cut = Math.floor(l.length / 2)
        await appendFile(file, l.slice(0, cut))
        await p.step()
        await appendFile(file, l.slice(cut))
        await p.step(NOW + 1)
      }
      await writeFile(file, lines.slice(0, 3).join(''))
      await p.step()
      await appendFile(file, lines.slice(3, 8).join('') + lines[0]!.slice(0, 10))
      const last = await p.step()
      expect(last.signals.length).toBeGreaterThan(0)
      await p.close()
      tally.files++
      tally.lines += lines.length + 8
    }
    const all = path.join(dir, 'edge-all.jsonl')
    await writeFile(all, lines.join(''))
    const out = await whole(all)
    const text = JSON.stringify(out)
    expect(text).not.toContain('SECRET')
    expect(text).not.toContain('description')
    expect(out.signals.filter((s) => s.k === 'reply_text').every((s) => s.k === 'reply_text' && s.text.length <= 3000)).toBe(true)
  })

  it('matches on the seeded demo corpus (30 sessions × 6 subagents)', async () => {
    const root = path.join(dir, 'demo')
    execFileSync(process.execPath, [DEMO, `--root=${root}`, '--sessions=30', '--subagents=6', '--seed=8704', '--tick=5', '--duration=2', '--peers=10'], { stdio: 'ignore', timeout: 60_000 })
    const files: string[] = []
    const walk = async (d: string): Promise<void> => {
      for (const e of await readdir(d, { withFileTypes: true })) {
        const p = path.join(d, e.name)
        if (e.isDirectory()) await walk(p)
        else if (e.name.endsWith('.jsonl')) files.push(p)
      }
    }
    await walk(path.join(root, 'projects'))
    expect(files.length).toBeGreaterThan(30 * 7)
    let bytes = 0
    for (const f of files) {
      bytes += (await stat(f)).size
      const isSub = f.includes(`${path.sep}subagents${path.sep}`)
      await whole(f, { spans: { terminalId: 'demo', agentId: isSub ? path.basename(f, '.jsonl') : 'main' } })
      if (isSub) await whole(f, { lite: [] })
    }
    expect(bytes).toBeGreaterThan(100_000)
  }, 120_000)
})

const describeBench = BIN && process.env.CCM_T2_BENCH ? describe : describe.skip

function cpuSeconds(pid: number): number {
  if (process.platform === 'win32') {
    const out = execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', `(Get-Process -Id ${pid}).TotalProcessorTime.TotalSeconds`], { encoding: 'utf8' })
    return Number(out.trim().replace(',', '.'))
  }
  const [m, s] = execFileSync('ps', ['-o', 'time=', '-p', String(pid)], { encoding: 'utf8' }).trim().split(':').slice(-2)
  return Number(m) * 60 + Number(s)
}

describeBench('transcript signals: bench Node vs Rust', () => {
  it('parses the demo corpus', async () => {
    const root = process.env.CCM_T2_BENCH!
    const files: string[] = []
    const walk = async (d: string): Promise<void> => {
      for (const e of await readdir(d, { withFileTypes: true })) {
        const p = path.join(d, e.name)
        if (e.isDirectory()) await walk(p)
        else if (e.name.endsWith('.jsonl')) files.push(p)
      }
    }
    await walk(root)
    const t0 = process.cpuUsage()
    let n = 0
    for (const f of files) n += (await new TsReader(f, {}).read(NOW)).signals.length
    const tsCpu = process.cpuUsage(t0)
    const child = spawn(BIN!, ['transcript'], { stdio: ['pipe', 'pipe', 'ignore'] })
    let buf = ''
    let replies = 0
    let rustSignals = 0
    const done = new Promise<void>((resolve) => {
      child.stdout.setEncoding('utf8').on('data', (d: string) => {
        buf += d
        let i: number
        while ((i = buf.indexOf('\n')) >= 0) {
          const msg = JSON.parse(buf.slice(0, i)) as { signals?: unknown[] }
          buf = buf.slice(i + 1)
          if (msg.signals) rustSignals += msg.signals.length
          if (++replies === files.length * 3) resolve()
        }
      })
    })
    const base = cpuSeconds(child.pid!)
    const w0 = Date.now()
    files.forEach((f, i) => {
      child.stdin.write(`${JSON.stringify({ id: i * 3, op: 'signals.open', file: f })}\n`)
      child.stdin.write(`${JSON.stringify({ id: i * 3 + 1, op: 'signals.read', handle: i + 1 })}\n`)
      child.stdin.write(`${JSON.stringify({ id: i * 3 + 2, op: 'signals.close', handle: i + 1 })}\n`)
    })
    await done
    const wall = Date.now() - w0
    const rustCpu = cpuSeconds(child.pid!) - base
    child.stdin.end()
    console.log(`[bench] ${files.length} files, ${n} / ${rustSignals} signals; TS ${((tsCpu.user + tsCpu.system) / 1e6).toFixed(3)} CPU-s; Rust ${rustCpu.toFixed(3)} CPU-s (${wall} ms wall incl. JSON reply)`)
    expect(rustSignals).toBe(n)
  }, 600_000)
})
