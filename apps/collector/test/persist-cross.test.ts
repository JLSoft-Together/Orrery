import { execFileSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import os from 'node:os'
import path from 'node:path'
import type { PlanLimits, SessionRecord, TerminalSession } from '@ccm/shared'
import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import { AliasStore, MAX_NOTE_LENGTH } from '../src/aliases'
import { defaultDataDir } from '../src/datadir'
import { firstStartToday } from '../src/daystart'
import { FavoriteStore } from '../src/favorites'
import { LimitHistory } from '../src/forecast'
import { SessionHistory } from '../src/history'
import { ResponseTracker } from '../src/response'
import { SnoozeStore } from '../src/snooze'
import { StatusTimeline } from '../src/timeline'
import { Toaster, type ToastDeps } from '../src/toast'

function cliBinary(): string | null {
  if (process.env.CCM_PERSIST_CLI) return process.env.CCM_PERSIST_CLI
  const exe = process.platform === 'win32' ? 'persist-cli.exe' : 'persist-cli'
  for (const profile of ['debug', 'release']) {
    const file = path.resolve(__dirname, '../../core/target', profile, exe)
    if (existsSync(file)) return file
  }
  return null
}

const bin = process.env.CCM_STORE_REPLAY_SKIP === '1' ? null : cliBinary()
const NOON = new Date(2027, 0, 15, 12, 0, 0).getTime()
const json = (v: unknown): unknown => JSON.parse(JSON.stringify(v))
type Op = unknown[]

let root = ''
let n = 0
const fresh = (name: string): string => path.join(root, `${name}-${n++}`, `${name}.json`)

function rust(kind: string, file: string, now: number, ops?: Op[]): unknown {
  const args = [kind, file, String(now)]
  if (ops) args.push(JSON.stringify(ops))
  return JSON.parse(execFileSync(bin as string, args, { encoding: 'utf8', windowsHide: true }))
}

const quietDeps: ToastDeps = { show: async () => true, hide: async () => true, now: () => NOON }
const toaster = (file: string): Toaster => new Toaster({ supported: true, port: 1, file }, quietDeps)

const terminal = (id: string, since: string): TerminalSession => ({ id, title: id, status: 'waiting', statusSince: since }) as TerminalSession

describe.skipIf(!bin)('Rust persist stores read what Node wrote and Node reads what Rust wrote', () => {
  beforeAll(async () => {
    root = await mkdtemp(path.join(os.tmpdir(), 'ccm-persist-cross-'))
  })
  afterAll(async () => {
    await rm(root, { recursive: true, force: true })
  })

  it.each([
    ['aliases', 80, 'Alpha  \t Beta', 'x'.repeat(200)],
    ['notes', MAX_NOTE_LENGTH, 'Ghi chú tiếng Việt', 'y'.repeat(300)],
  ] as const)('%s', async (kind, max, first, long) => {
    const a = fresh(kind)
    const node = new AliasStore(a, max)
    node.set('t1', first.replace(/\s+/g, ' '))
    node.set('t2', long.slice(0, max))
    node.set('t3', 'gone')
    node.delete('t3')
    await (node as unknown as { saving: Promise<void> }).saving
    expect(rust(kind, a, NOON)).toEqual({ t1: node.get('t1'), t2: node.get('t2') })

    const b = fresh(kind)
    const view = rust(kind, b, NOON, [['set', 't1', first], ['set', 't2', long], ['set', 't3', 'gone'], ['delete', 't3'], ['set', 't4', null]]) as Record<string, string>
    const again = new AliasStore(b, max)
    await again.load()
    expect(Object.keys(view)).toEqual(['t1', 't2'])
    for (const [id, alias] of Object.entries(view)) expect(again.get(id)).toBe(alias)
    expect(again.get('t3')).toBeUndefined()
  })

  it('favorites', async () => {
    const dirs = ['alpha', 'bravo', 'charlie', 'delta'].map((d) => path.join(root, 'fav', d))
    const a = fresh('favorites')
    const node = new FavoriteStore(a)
    node.toggle(dirs[0]!, 'Alpha')
    node.add(dirs[1]!, 'Bravo  team')
    node.add(dirs[2]!, undefined)
    node.add(dirs[3]!, 'Delta')
    node.rename(dirs[3]!, 'Dạ lan')
    node.reorder([dirs[3]!, dirs[0]!])
    await node.whenSaved()
    expect(rust('favorites', a, NOON)).toEqual(json(node.list()))

    const b = fresh('favorites')
    const view = rust('favorites', b, NOON, [['toggle', dirs[0], 'Alpha'], ['add', dirs[1], 'Bravo  team'], ['add', dirs[2], null], ['add', dirs[3], 'Delta'], ['rename', dirs[3], 'Dạ lan'], ['reorder', [dirs[3], dirs[0]]], ['remove', dirs[1]]]) as { label: string }[]
    const again = new FavoriteStore(b)
    await again.load()
    expect(json(again.list())).toEqual(view)
    expect(view.map((f) => f.label)).toEqual(['Dạ lan', 'Alpha', 'charlie'])
  })

  it('history', async () => {
    const records: SessionRecord[] = [
      { id: 's1', title: 'One', endedAt: '2027-01-15T10:00:00.000Z', inputTokens: 5, outputTokens: 6, cacheReadTokens: 7, subagents: 1, cwd: '/p', model: 'opus', costUsd: 1.25, workingMs: 1000 },
      { id: 's2', title: 'Two', endedAt: '2027-01-15T11:00:00.000Z', inputTokens: 0, outputTokens: 0, cacheReadTokens: 0, subagents: 0, compactions: 2, sessionId: 'abc' },
      { id: 's1', title: 'One again', endedAt: '2027-01-15T12:00:00.000Z', inputTokens: 1, outputTokens: 2, cacheReadTokens: 3, subagents: 0 },
    ]
    const a = fresh('history')
    const node = new SessionHistory(a)
    for (const r of records) node.add(r)
    await node.save()
    expect(rust('history', a, NOON)).toEqual(json(node.list()))

    const b = fresh('history')
    const view = rust('history', b, NOON, records.map((r) => ['add', r]))
    const again = new SessionHistory(b)
    await again.load()
    expect(json(again.list())).toEqual(view)
    expect((view as { id: string }[]).map((r) => r.id)).toEqual(['s1', 's2'])
  })

  it('snooze', async () => {
    const a = fresh('snooze')
    const node = new SnoozeStore(a, () => NOON)
    node.set(terminal('w1', 's1'), 5)
    node.set(terminal('w2', 's2'), 90)
    node.set(terminal('w3', 's3'), 10)
    node.clear('w3')
    await node.flush()
    expect(rust('snooze', a, NOON + 60_000)).toEqual(json(node.all()))

    const b = fresh('snooze')
    const view = rust('snooze', b, NOON, [['set', terminal('w1', 's1'), 5], ['set', terminal('w2', 's2'), 90], ['set', terminal('w3', 's3'), 10], ['clear', 'w3'], ['set', terminal('w4', 's4'), 99999]])
    const again = new SnoozeStore(b, () => NOON)
    await again.load()
    expect(json(again.all())).toEqual(view)
    expect(Object.keys(view as object)).toEqual(['w1', 'w2'])
  })

  it('response stats', async () => {
    const waits = [1000, 400_000.4, 3000, 2000, -5, 9 * 3_600_000]
    const a = fresh('response')
    const node = new ResponseTracker(a, () => NOON)
    for (const w of waits) node.record(w)
    await node.save()
    expect(rust('response', a, NOON + 1000)).toEqual(json(node.stats()))

    const b = fresh('response')
    const view = rust('response', b, NOON, waits.map((w) => ['record', w]))
    const again = new ResponseTracker(b, () => NOON)
    await again.load()
    expect(json(again.stats())).toEqual(view)
    expect((view as { count: number }).count).toBe(4)
  })

  it('status timeline', async () => {
    const at = (min: number): string => new Date(NOON - min * 60_000).toISOString()
    let now = NOON - 3_600_000
    const a = fresh('timeline')
    const node = new StatusTimeline(a, () => now)
    node.observe('t1', 'One', 'working', at(60))
    now += 20 * 60_000
    node.observe('t1', 'One', 'waiting')
    node.observe('t2', 'Two', 'idle')
    now += 20 * 60_000
    node.observe('t1', 'Uno', 'working')
    now += 5 * 60_000
    await node.save()
    now = NOON
    const nodeLoaded = new StatusTimeline(a, () => NOON)
    await nodeLoaded.load()
    expect(rust('timeline', a, NOON)).toEqual(json(nodeLoaded.data()))

    const b = fresh('timeline')
    const ops: Op[] = [['observe', 't1', 'One', 'working', at(50)], ['observe', 't1', 'One', 'waiting', at(30)], ['observe', 't2', 'Two', 'idle', null], ['observe', 't1', 'Uno', 'error', at(10)], ['observe', 't2', 'Two', 'stale', at(5)], ['observe', 't3', 'Three', 'stale', null]]
    const view = rust('timeline', b, NOON, ops)
    const again = new StatusTimeline(b, () => NOON)
    await again.load()
    expect(json(again.data())).toEqual(view)
    expect((view as { lanes: unknown[] }).lanes).toHaveLength(2)
  })

  it('limit history and forecast', async () => {
    const limits = (min: number, used: number, resets = '2027-01-15T17:00:00.000Z'): PlanLimits => ({
      updatedAt: new Date(NOON - 3_600_000 + min * 60_000).toISOString(),
      fiveHour: { usedPct: used, resetsAt: resets },
      sevenDay: { usedPct: used / 4, resetsAt: '2027-01-20T00:00:00.000Z' },
    })
    const series = [limits(0, 10), limits(0.5, 11), limits(15, 21), limits(30, 31.5), limits(45, 40)]
    const probe = limits(60, 52.5)
    const a = fresh('limits')
    const node = new LimitHistory(a)
    for (const l of series) node.apply(l)
    await node.save()
    const nodeProbe = json(node.apply(probe))
    expect(rust('limits', a, NOON, [['probe', probe]])).toEqual(nodeProbe)

    const b = fresh('limits')
    const view = rust('limits', b, NOON, [...series.map((l) => ['apply', l]), ['probe', probe]])
    const again = new LimitHistory(b)
    await again.load()
    expect(json(again.apply(probe))).toEqual(view)
    expect(nodeProbe).toEqual(view)
    expect((view as PlanLimits).fiveHour?.forecast?.pctPerHour).toBeGreaterThan(0)
  })

  it('notify.json', async () => {
    const a = fresh('notify')
    const node = toaster(a)
    await node.load()
    node.update({ toast: false, kinds: { done: true, loop: false }, quiet: { enabled: true, from: '23:15', to: '06:45' }, stuckMinutes: 5, idleMinutes: 240 })
    await node.flush()
    expect(rust('notify', a, NOON)).toEqual(JSON.parse(await readFile(a, 'utf8')))

    const b = fresh('notify')
    const view = rust('notify', b, NOON, [['patch', { toast: false, locale: 'en', kinds: { done: true, loop: false, bogus: true }, quiet: { enabled: true, from: '23:15', to: '06:45' }, stuckMinutes: 5, idleMinutes: 240 }]]) as Record<string, unknown>
    const again = toaster(b)
    await again.load()
    expect(again.key).toBe(view.key)
    expect(again.locale).toBe('en')
    expect(again.settings()).toEqual({ toast: 'off', kinds: view.kinds, quiet: view.quiet, stuckMinutes: view.stuckMinutes, idleMinutes: view.idleMinutes })
    await writeFile(b, '{"toast":"yes","stuckMinutes":7}')
    const odd = toaster(b)
    await odd.load()
    expect(odd.settings().stuckMinutes).toBe(10)
  })

  it('daystart', async () => {
    const a = fresh('day-start')
    const first = await firstStartToday(a, NOON)
    expect(rust('daystart', a, NOON + 3 * 3_600_000)).toBe(first)
    const b = fresh('day-start')
    const second = rust('daystart', b, NOON) as string
    expect(await firstStartToday(b, NOON + 3 * 3_600_000)).toBe(second)
    expect(second).toBe(new Date(NOON).toISOString())
  })

  it.each([
    ['win32', { LOCALAPPDATA: 'C:\\Users\\a\\AppData\\Local' }, 'C:\\Users\\a'],
    ['win32', {}, 'C:\\Users\\a'],
    ['win32', { LOCALAPPDATA: '' }, 'D:/Users/b'],
    ['darwin', {}, '/Users/a'],
  ] as const)('datadir %s %j', (platform, env, home) => {
    const local = (env as { LOCALAPPDATA?: string }).LOCALAPPDATA
    const out = JSON.parse(execFileSync(bin as string, ['datadir', platform, local === undefined ? '-' : local === '' ? '' : local, home], { encoding: 'utf8', windowsHide: true }))
    expect(out).toBe(defaultDataDir(env, platform, home))
  })
})
