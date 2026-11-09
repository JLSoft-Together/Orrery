import { execFileSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import { appendFile, mkdir, mkdtemp, readdir, readFile, rm, writeFile } from 'node:fs/promises'
import os from 'node:os'
import path from 'node:path'
import { afterAll, describe, expect, it } from 'vitest'
import { createCoreTranscript, type CoreTranscriptClient } from '../src/coreTranscript'
import type { RegistryRecord } from '../src/registry'
import { SessionTracker, terminalIdOf } from '../src/tracker'
import { createCoreHub } from '../src/trackerSource'
import { BIN, Twin, tally } from './tracker-twin'

const DEMO = path.resolve(__dirname, '../../../scripts/demo-fixture.mjs')
const STEPS = 20
const HOUR = 3_600_000
const BASE = 1_800_000_000_000

interface Source {
  rel: string
  lines: string[]
}

async function walk(d: string, out: string[] = []): Promise<string[]> {
  for (const e of await readdir(d, { withFileTypes: true })) {
    const p = path.join(d, e.name)
    if (e.isDirectory()) await walk(p, out)
    else out.push(p)
  }
  return out
}

let client: CoreTranscriptClient | null = null
let dir = ''

const describeCore = BIN ? describe : describe.skip

describeCore('SessionTracker: Node ↔ Rust parity on the seeded demo (30 sessions × 6 subagents)', () => {
  afterAll(async () => {
    client?.dispose()
    if (dir) await rm(dir, { recursive: true, force: true })
  })

  it('matches after every replay step', async () => {
    dir = await mkdtemp(path.join(os.tmpdir(), 'ccm-t3-demo-'))
    const root = path.join(dir, 'src')
    execFileSync(process.execPath, [DEMO, `--root=${root}`, '--sessions=30', '--subagents=6', '--seed=8704', '--tick=5', '--duration=1', '--peers=10'], { stdio: 'ignore', timeout: 60_000 })
    const projects = path.join(root, 'projects')
    const live = path.join(dir, 'projects')
    const files = await walk(projects)
    const jsonl: Source[] = []
    const metas = new Map<string, { rel: string; text: string; toolUseId?: string }>()
    for (const f of files) {
      const rel = path.relative(projects, f)
      const text = await readFile(f, 'utf8')
      if (f.endsWith('.jsonl')) jsonl.push({ rel, lines: text.split(/(?<=\n)/).filter(Boolean) })
      else if (f.endsWith('.meta.json')) metas.set(rel, { rel, text, toolUseId: (JSON.parse(text) as { toolUseId?: string }).toolUseId })
    }
    const registry: RegistryRecord[] = []
    for (const f of await readdir(path.join(root, 'sessions'))) registry.push(JSON.parse(await readFile(path.join(root, 'sessions', f), 'utf8')) as RegistryRecord)
    expect(registry).toHaveLength(30)

    const copied = new Map<string, number>()
    const placed = new Set<string>()
    const seen = new Set<string>()
    const step = async (k: number): Promise<void> => {
      for (const src of jsonl) {
        if (src.rel.includes(`${path.sep}subagents${path.sep}`) && !placed.has(src.rel)) continue
        const upto = Math.ceil((src.lines.length * k) / STEPS)
        const from = copied.get(src.rel) ?? 0
        const target = path.join(live, src.rel)
        if (from === 0) await mkdir(path.dirname(target), { recursive: true })
        if (upto > from || !existsSync(target)) {
          const chunk = src.lines.slice(from, upto).join('')
          await appendFile(target, chunk)
          for (const m of chunk.matchAll(/"id":"(toolu_[^"]+)","name":"Agent"/g)) seen.add(m[1]!)
          copied.set(src.rel, upto)
        }
      }
      for (const meta of metas.values()) {
        if (placed.has(meta.rel.replace('.meta.json', '.jsonl')) || !meta.toolUseId || !seen.has(meta.toolUseId)) continue
        const target = path.join(live, meta.rel)
        await mkdir(path.dirname(target), { recursive: true })
        await writeFile(target, meta.text)
        placed.add(meta.rel.replace('.meta.json', '.jsonl'))
      }
    }

    client = createCoreTranscript({ bin: BIN!, timeoutMs: 60_000 })
    const hub = createCoreHub(() => client)
    const proto = SessionTracker.prototype as unknown as { isLiteCatchUp: (e: unknown) => Promise<boolean> }
    const real = proto.isLiteCatchUp
    let lite = 0
    proto.isLiteCatchUp = async function (this: unknown, e: unknown) {
      const r = await real.call(this, e)
      if (r) lite++
      return r
    }
    let now = BASE + HOUR
    const twins: (Twin | null)[] = registry.map(() => null)
    const late = STEPS / 2
    let activity = 0
    for (let k = 1; k <= STEPS; k++) {
      await step(k)
      if (k === late) now = BASE + 3 * HOUR
      for (const [i, r] of registry.entries()) {
        if (twins[i] || (i % 2 === 1 && k < late)) continue
        twins[i] = new Twin(terminalIdOf(r), r, live, {}, () => now, hub)
        await twins[i]!.sync(true)
      }
      if (k === 1) continue
      for (const [i, t] of twins.entries()) {
        if (!t) continue
        const scope = k % 3 === 0 ? { skipResolved: true } : k % 3 === 1 ? undefined : { files: [`${registry[i]!.sessionId}.jsonl`] }
        activity += (await t.sync(false, scope)).length
        t.drainSpans()
        t.drainPeers()
      }
    }
    for (const t of twins) await t!.sync(false)
    proto.isLiteCatchUp = real
    expect(lite).toBeGreaterThan(10)
    expect(activity).toBeGreaterThan(1000)
    expect(placed.size).toBeGreaterThan(30 * 6)
    console.log(`[tracker demo parity] ${twins.length} sessions, ${placed.size} subagents, ${STEPS} steps, ${lite} lite catch-ups, ${activity} live activity, ${tally.syncs} syncs`)
  }, 300_000)
})
