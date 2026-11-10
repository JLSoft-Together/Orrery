import { mkdtemp, rm, writeFile } from 'node:fs/promises'
import os from 'node:os'
import path from 'node:path'
import { afterAll, describe, expect, it } from 'vitest'
import { NodeToast, RustToast, split, TOAST_BIN, type Cmd } from './toast-twin'

const T0 = Date.parse('2026-10-08T10:00:00')
const KEY = '0123456789abcdef0123456789abcdef'
const GB = 1024 ** 3
const iso = (ms: number) => new Date(ms).toISOString()

function rng(seed: number) {
  let s = seed >>> 0
  const next = () => {
    s = (Math.imul(s, 1664525) + 1013904223) >>> 0
    return s / 2 ** 32
  }
  return { next, int: (n: number) => Math.floor(next() * n), pick: <T>(xs: readonly T[]) => xs[Math.floor(next() * xs.length)] as T, chance: (p: number) => next() < p }
}

function scripted(): Cmd[] {
  const a = (status: string, extra: Record<string, unknown> = {}) => ({ id: 'a', title: 'title-a', status, cwd: 'D:\\work\\shop', ...extra })
  const ask = { question: 'Which?', options: ['A', 'B'], multi: false }
  const limits = (pct: number, inMin: number) => ({ updatedAt: iso(T0), fiveHour: { usedPct: pct, resetsAt: iso(T0 + 4.5 * 3600_000), forecast: { pctPerHour: 40, fullAt: iso(T0 + inMin * 60_000), beforeReset: true } } })
  const hit = { at: iso(T0), resetsAt: iso(T0 + 3600_000), window: 'five_hour' }
  const ev = (i: number, kind: string) => ({ id: `${kind}${i}`, at: iso(T0 - 60_000 + i * 1000), kind, terminalId: 'a', data: { tool: 'Bash' } })
  return [
    { cmd: 'observe', terminals: [a('working')], limits: null },
    { cmd: 'observe', terminals: [a('waiting', { waitingFor: 'input needed', statusSince: iso(T0) })], toolOf: { a: 'AskUserQuestion' }, askOf: { a: ask }, limits: limits(50, 120) },
    { cmd: 'advance', ms: 5 * 60_000 + 1 },
    { cmd: 'observe', terminals: [a('waiting', { waitingFor: 'input needed', statusSince: iso(T0) })], limits: limits(82, 30) },
    { cmd: 'snooze', ids: ['a'] },
    { cmd: 'advance', ms: 5 * 60_000 },
    { cmd: 'observe', terminals: [a('waiting', { waitingFor: 'input needed', statusSince: iso(T0) })] },
    { cmd: 'snooze', ids: [] },
    { cmd: 'observe', terminals: [a('working', { statusSince: iso(T0 + 10 * 60_000), lastActivityAt: iso(T0) }), { id: 'b', title: 'B', status: 'working' }], contextPct: { a: 95.5 } },
    { cmd: 'update', patch: { stuckMinutes: 5, kinds: { done: true } } },
    { cmd: 'observe', terminals: [a('working', { statusSince: iso(T0 + 10 * 60_000), lastActivityAt: iso(T0) })], contextPct: { a: 96 } },
    { cmd: 'loops', events: [0, 1, 2, 3, 4, 5].flatMap((i) => [ev(i, 'tool.started'), ev(i, 'tool.failed')]) },
    { cmd: 'advance', ms: 6 * 60_000 },
    { cmd: 'observe', terminals: [a('idle', { statusSince: iso(T0 + 16 * 60_000), limitHit: hit })], replyOf: { a: 'Done: all tests pass' } },
    { cmd: 'sensitive', event: { id: 'x', at: iso(T0), kind: 'sensitive', terminalId: 'a', data: { rule: 'rm-rf', tool: 'Bash', path: 'C:\\tmp' } } },
    { cmd: 'disk', msg: { type: 'disk.alert', label: 'C:', level: 'danger', freeBytes: 1.25 * GB, totalBytes: 500 * GB } },
    { cmd: 'note', msg: { type: 'agent.note', episode: 'n1', level: 'attention', text: 'Pick one', name: 'api', terminalId: 'a', merged: 2 } },
    { cmd: 'idle', report: { idleMinutes: 60, totals: { forgottenRamBytes: 0.3 * GB }, sessions: [], servers: [] } },
    { cmd: 'idle', report: { idleMinutes: 60, totals: { forgottenRamBytes: 0.3 * GB }, sessions: [{ sessionId: 'a', forgotten: true, idleSinceMs: 7 }], servers: [{ pid: 9, start: '1', name: 'node.exe', forgotten: true, orphan: true }] } },
    { cmd: 'observe', terminals: [a('working')], jobs: [{ id: 'j', state: 'working', sessionId: 's' }] },
    { cmd: 'observe', terminals: [a('working', { claudeSessionId: 's' })], jobs: [{ id: 'j', name: 'nightly', state: 'blocked', sessionId: 's', stateSince: iso(T0) }] },
    { cmd: 'presence', clientId: 1, attentive: true, locale: 'en' },
    { cmd: 'presence', clientId: 1, attentive: false },
    { cmd: 'observe', terminals: [a('waiting', { waitingFor: 'permission prompt', statusSince: iso(T0 + 20 * 60_000) })], toolOf: { a: 'Bash' } },
    { cmd: 'update', patch: { quiet: { enabled: true, from: '00:00', to: '23:59' } } },
    { cmd: 'disk', msg: { type: 'disk.alert', label: 'D:', level: 'danger', freeBytes: 1, totalBytes: 2 } },
    { cmd: 'update', patch: { quiet: { enabled: false, from: '22:00', to: '08:00' }, kinds: { waiting: false } } },
    { cmd: 'update', patch: { toast: false } },
    { cmd: 'settings' },
    { cmd: 'state' },
    { cmd: 'test' },
    { cmd: 'accepts', key: KEY },
    { cmd: 'accepts', key: 'nope' },
    { cmd: 'link', id: 'a b/é' },
    { cmd: 'snoozeLink', id: '42:7' },
    { cmd: 'observe', terminals: [] },
    { cmd: 'clear' },
  ]
}

function fuzz(seed: number, steps: number): Cmd[] {
  const r = rng(seed)
  let now = T0
  const ids = ['a', 'b', 'c', 'd']
  const live = new Map<string, Record<string, unknown>>()
  const out: Cmd[] = []
  const stamp = (t: Record<string, unknown>, status: string) => {
    if (t.status !== status) t.statusSince = iso(now - r.int(3) * 60_000)
    t.status = status
  }
  for (let i = 0; i < steps; i++) {
    const roll = r.next()
    if (roll < 0.12) {
      const ms = r.pick([0, 1000, 30_000, 60_000, 5 * 60_000, 7 * 60_000, 3600_000])
      now += ms
      out.push({ cmd: 'advance', ms })
    } else if (roll < 0.62) {
      for (const id of ids) {
        if (!live.has(id) && r.chance(0.5)) live.set(id, { id, title: `title-${id}`, status: 'idle', ...(r.chance(0.6) ? { cwd: `D:\\work\\${id}` } : {}), ...(r.chance(0.3) ? { alias: `Alias ${id}` } : {}) })
        const t = live.get(id)
        if (!t) continue
        if (r.chance(0.05)) {
          live.delete(id)
          continue
        }
        if (r.chance(0.5)) stamp(t, r.pick(['working', 'working', 'waiting', 'idle']))
        t.waitingFor = t.status === 'waiting' ? r.pick([undefined, 'permission prompt', 'input needed', 'dialog open', 'foo']) : undefined
        if (r.chance(0.3)) t.lastActivityAt = iso(now - r.int(20) * 60_000)
        if (r.chance(0.05)) t.limitHit = { at: iso(now), resetsAt: iso(now + r.pick([-1, 1, 5]) * 3600_000), window: r.pick(['five_hour', 'seven_day', 'other']) }
        if (r.chance(0.03)) t.endedAt = iso(now)
        if (r.chance(0.2)) t.claudeSessionId = `s-${id}`
      }
      const terms = [...live.values()].map((t) => JSON.parse(JSON.stringify(t)))
      const cmd: Cmd = { cmd: 'observe', terminals: terms }
      if (r.chance(0.5)) cmd.contextPct = Object.fromEntries(terms.map((t) => [t.id, r.pick([10, 94.5, 95, 95.5, 99])]))
      if (r.chance(0.5)) cmd.toolOf = Object.fromEntries(terms.map((t) => [t.id, r.pick(['Bash', 'ExitPlanMode', 'AskUserQuestion', ''])]))
      if (r.chance(0.4)) cmd.replyOf = Object.fromEntries(terms.map((t) => [t.id, `reply ${t.id}`]))
      if (r.chance(0.4)) cmd.askOf = Object.fromEntries(terms.map((t) => [t.id, { question: 'Q?', options: ['x'], multi: false }]))
      if (r.chance(0.5)) cmd.jobs = ids.slice(0, r.int(4)).map((id) => ({ id: `j${id}`, state: r.pick(['working', 'blocked', 'done']), sessionId: `s-${id}`, stateSince: iso(now - 60_000), ...(r.chance(0.5) ? { name: `job ${id}` } : {}) }))
      if (r.chance(0.4)) {
        const pct = r.pick([40, 79.5, 85, 100])
        cmd.limits = r.chance(0.2) ? null : { updatedAt: iso(now), fiveHour: { usedPct: pct, resetsAt: iso(T0 + r.int(3) * 5 * 3600_000), forecast: { pctPerHour: 40, fullAt: iso(now + r.pick([10, 50, 90]) * 60_000), beforeReset: r.chance(0.8) } } }
      }
      out.push(cmd)
    } else if (roll < 0.68) {
      out.push({ cmd: 'snooze', ids: ids.filter(() => r.chance(0.3)) })
    } else if (roll < 0.74) {
      out.push({ cmd: 'presence', clientId: r.int(2), attentive: r.chance(0.3), ...(r.chance(0.3) ? { locale: r.pick(['en', 'vi']) } : {}) })
    } else if (roll < 0.8) {
      const patch: Record<string, unknown> = {}
      if (r.chance(0.2)) patch.toast = r.chance(0.7)
      if (r.chance(0.5)) patch.kinds = { [r.pick(['waiting', 'done', 'job', 'loop', 'stuck', 'context', 'limit', 'idle', 'sensitive', 'disk'])]: r.chance(0.5) }
      if (r.chance(0.2)) patch.quiet = { enabled: r.chance(0.5), from: r.pick(['09:00', '22:00', '10:00']), to: r.pick(['11:00', '08:00', '10:00']) }
      if (r.chance(0.2)) patch.stuckMinutes = r.pick([0, 5, 10, 7])
      if (r.chance(0.2)) patch.idleMinutes = r.pick([0, 15, 60, 3])
      out.push({ cmd: 'update', patch })
    } else if (roll < 0.85) {
      const id = r.pick(ids)
      const events = Array.from({ length: r.int(8) }, (_, k) => ({ id: `e${i}-${k}`, at: iso(now - r.int(400) * 1000), kind: r.pick(['tool.started', 'tool.failed', 'tool.failed']), terminalId: id, data: { tool: r.pick(['Bash', 'Edit']) } }))
      out.push({ cmd: 'loops', events })
    } else if (roll < 0.88) {
      out.push({ cmd: 'sensitive', event: { id: `x${i}`, at: iso(now), kind: 'sensitive', terminalId: r.pick(ids), data: { rule: r.pick(['rm-rf', 'custom:*.pem', 'git-clean']), tool: 'Bash', ...(r.chance(0.5) ? { path: 'C:\\x' } : {}) } } })
    } else if (roll < 0.91) {
      out.push({ cmd: 'disk', msg: { type: 'disk.alert', label: r.pick(['C:', 'D:']), level: r.pick(['warn', 'danger']), freeBytes: r.pick([1.25, 8, 150.5]) * GB, totalBytes: 500 * GB } })
    } else if (roll < 0.94) {
      out.push({ cmd: 'note', msg: { type: 'agent.note', episode: `n${i}`, level: r.pick(['info', 'attention', 'done']), text: 'note text', name: 'pane', merged: r.int(3), ...(r.chance(0.6) ? { terminalId: r.pick(ids) } : {}) } })
    } else if (roll < 0.97) {
      const sessions = ids.filter(() => r.chance(0.4)).map((id) => ({ sessionId: id, forgotten: r.chance(0.7), idleSinceMs: r.pick([1, 2]) }))
      const servers = r.chance(0.5) ? [{ pid: 7, start: '1', name: 'java.exe', forgotten: true, idleSinceMs: r.pick([1, 2]), daemon: { type: 'gradle', version: '8.14' } }] : []
      out.push({ cmd: 'idle', report: { idleMinutes: r.pick([0, 60]), totals: { forgottenRamBytes: r.pick([0.25, 1.75, 3]) * GB }, sessions, servers } })
    } else {
      out.push(r.pick<Cmd>([{ cmd: 'hideFor', id: r.pick(ids) }, { cmd: 'clear' }, { cmd: 'settings' }, { cmd: 'state' }, { cmd: 'test' }]))
    }
  }
  return out
}

describe.skipIf(!TOAST_BIN)('Rust toast twin (core-toast vs Node Toaster on one event sequence)', () => {
  const dirs: string[] = []
  afterAll(async () => {
    for (const d of dirs) await rm(d, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 })
  })

  const cases: [string, Cmd[]][] = [['scripted', scripted()], ...[1, 2, 3, 4, 5, 6].map((s): [string, Cmd[]] => [`fuzz seed ${s}`, fuzz(s, 400)])]
  for (const [name, steps] of cases) {
    it(`gives the same toasts, phone and Telegram episodes: ${name}`, async () => {
      const dir = await mkdtemp(path.join(os.tmpdir(), 'ccm-toast-twin-'))
      dirs.push(dir)
      const files = [path.join(dir, 'node.json'), path.join(dir, 'rust.json')]
      for (const f of files) await writeFile(f, JSON.stringify({ key: KEY }))
      const node = new NodeToast()
      const rust = new RustToast(TOAST_BIN!)
      let episodes = 0
      let toasts = 0
      try {
        const init = { cmd: 'init', now: T0, port: 4317, phone: true, telegram: true }
        expect(await node.run({ ...init, file: files[0] })).toEqual(await rust.run({ ...init, file: files[1] }))
        for (const [i, step] of steps.entries()) {
          const [n, r] = [await node.run(step), await rust.run(step)]
          const [sn, sr] = [split(n.out), split(r.out)]
          const where = `${name} step ${i} ${step.cmd}`
          expect(r.result, where).toEqual(n.result)
          expect(sr.episodes, where).toEqual(sn.episodes)
          expect(sr.byTag, where).toEqual(sn.byTag)
          episodes += sn.episodes.length
          toasts += n.out.length - sn.episodes.length
        }
      } finally {
        rust.close()
      }
      expect(episodes + toasts).toBeGreaterThan(5)
      console.log(`[toast-twin] ${name}: ${steps.length} steps, ${toasts} toast ops, ${episodes} phone/Telegram episodes, 0 diffs`)
    }, 60_000)
  }
})
