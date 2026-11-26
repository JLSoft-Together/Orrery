import { describe, expect, it } from 'vitest'
import { nextTick, reactive } from 'vue'
import type { CheckState, ChecklistCheck, ChecklistItem } from '../../../core/lib/preview'
import type { Pane } from '../../workspace/stores/workspace'
import { checkView, defaultScript, isCheckLive, mergeWarning, onCheckDone, openItems, runCheck, scriptOptions, startCheckFromPhone, waitShellResult, type PhoneRunDeps } from './checklist'

const item = (id: string, done: boolean, by: 'owner' | 'agent' = 'owner'): ChecklistItem => ({ id, text: `text ${id}`, done, by, at: 1 })
const check = (state: CheckState, head = 'aaa1111', script = 'npm run qa'): ChecklistCheck => ({ script, state, at: 5, head })

describe('checkView', () => {
  it('has no view without a script', () => {
    expect(checkView(check('pass', 'a', ''), 'a', false)).toBe('none')
  })
  it('keeps a result only for the commit it ran on, otherwise it is out of date', () => {
    expect(checkView(check('pass', 'aaa1111'), 'aaa1111', false)).toBe('pass')
    expect(checkView(check('fail', 'aaa1111'), 'aaa1111', false)).toBe('fail')
    expect(checkView(check('pass', 'aaa1111'), 'bbb2222', false)).toBe('stale')
    expect(checkView(check('fail', 'aaa1111'), 'bbb2222', false)).toBe('stale')
  })
  it('shows running only while a run is alive in this app', () => {
    expect(checkView(check('running'), 'aaa1111', true)).toBe('running')
    expect(checkView(check('running'), 'aaa1111', false)).toBe('idle')
    expect(checkView(check('idle', ''), 'aaa1111', false)).toBe('idle')
  })
})

describe('mergeWarning', () => {
  it('is quiet when everything is done and the check is not failing', () => {
    expect(mergeWarning([item('t1', true)], 'pass')).toBeNull()
    expect(mergeWarning([], 'stale')).toBeNull()
  })
  it('counts open items and keeps the first three texts', () => {
    const w = mergeWarning([item('t1', false), item('t2', true), item('t3', false), item('t4', false), item('t5', false)], 'idle')
    expect(w).toEqual({ open: 4, texts: ['text t1', 'text t3', 'text t4'], failed: false })
    expect(openItems([item('t1', false), item('t2', true)])).toHaveLength(1)
  })
  it('warns about a failing check even with no open items', () => {
    expect(mergeWarning([], 'fail')).toEqual({ open: 0, texts: [], failed: true })
  })
})

describe('scripts', () => {
  const groups = [
    { source: 'npm' as const, file: 'package.json', items: [{ name: 'build', run: 'npm run build' }, { name: 'qa', run: 'npm run qa' }] },
    { source: 'make' as const, file: 'Makefile', items: [{ name: 'qa', run: 'make qa' }] },
  ]
  it('flattens the groups and prefers qa', () => {
    const options = scriptOptions(groups)
    expect(options.map((o) => o.run)).toEqual(['npm run build', 'npm run qa', 'make qa'])
    expect(defaultScript(options)).toBe('npm run qa')
    expect(defaultScript(scriptOptions([groups[0]!].map((g) => ({ ...g, items: [g.items[0]!] }))))).toBe('')
  })
})

describe('runCheck', () => {
  function deps(result: { code: number } | null, head = 'abc1234') {
    const reports: [CheckState, string][] = []
    const launched: string[] = []
    return {
      reports,
      launched,
      deps: {
        head: async () => head,
        report: async (s: CheckState, h: string) => void reports.push([s, h]),
        launch: (script: string) => (launched.push(script), 'pane-1'),
        wait: async () => result,
      },
    }
  }
  it('reports running, then pass for exit 0, both stamped with the starting HEAD', async () => {
    const d = deps({ code: 0 })
    expect(await runCheck('/wt/a', 'npm run qa', d.deps)).toBe('pass')
    expect(d.launched).toEqual(['npm run qa'])
    expect(d.reports).toEqual([['running', 'abc1234'], ['pass', 'abc1234']])
  })
  it('stores idle, not a stuck running, when launching the script throws', async () => {
    const d = deps({ code: 0 })
    const boom = { ...d.deps, launch: () => { throw new Error('no pane') } }
    const seen: [CheckState, string][] = []
    const out = await runCheck('/wt/boom2', 'x', { ...boom, report: async (s, h) => void seen.push([s, h]) })
    expect(out).toBe('idle')
    expect(seen).toEqual([['running', 'abc1234'], ['idle', 'abc1234']])
    expect(isCheckLive('/wt/boom2')).toBe(false)
  })
  it('tells subscribers when the run ends, until they unsubscribe', async () => {
    let n = 0
    const off = onCheckDone('/wt/f', () => n++)
    await runCheck('/wt/f', 'x', deps({ code: 0 }).deps)
    expect(n).toBe(1)
    off()
    await runCheck('/wt/f', 'x', deps({ code: 0 }).deps)
    expect(n).toBe(1)
  })
  it('reports fail for a non-zero exit', async () => {
    const d = deps({ code: 2 })
    expect(await runCheck('/wt/b', 'npm run qa', d.deps)).toBe('fail')
    expect(d.reports.at(-1)).toEqual(['fail', 'abc1234'])
  })
  it('goes back to idle when the terminal was closed before a result', async () => {
    const d = deps(null)
    expect(await runCheck('/wt/c', 'npm run qa', d.deps)).toBe('idle')
    expect(d.reports.at(-1)).toEqual(['idle', 'abc1234'])
  })
  it('does nothing without HEAD or a script, and refuses a second run in the same worktree', async () => {
    const none = deps({ code: 0 }, '')
    expect(await runCheck('/wt/d', 'x', none.deps)).toBe('idle')
    expect(none.launched).toEqual([])
    expect(await runCheck('/wt/d', '  ', deps({ code: 0 }).deps)).toBe('idle')
    let release: (v: { code: number }) => void = () => {}
    const slow = deps({ code: 0 })
    slow.deps.wait = () => new Promise((r) => (release = r))
    const first = runCheck('/wt/e', 'x', slow.deps)
    await Promise.resolve()
    await Promise.resolve()
    expect(isCheckLive('/wt/e')).toBe(true)
    const second = deps({ code: 0 })
    expect(await runCheck('/wt/e', 'x', second.deps)).toBe('idle')
    expect(second.launched).toEqual([])
    release({ code: 0 })
    expect(await first).toBe('pass')
    expect(isCheckLive('/wt/e')).toBe(false)
  })
})

describe('waitShellResult', () => {
  const pane = (over: Partial<Pane> = {}) => reactive({ key: 'p1', preset: 'shell', state: 'running', ...over }) as unknown as Pane
  it('resolves with the exit code the shell integration reports', async () => {
    const p = pane()
    const waiting = waitShellResult(() => [p], 'p1')
    await nextTick()
    p.shellLast = { code: 3, ms: 10, command: 'npm run qa' }
    expect(await waiting).toEqual({ code: 3 })
  })
  it('resolves null when the pane exits or disappears with no result', async () => {
    const p = pane()
    const list = reactive<Pane[]>([p])
    const waiting = waitShellResult(() => list, 'p1')
    list.splice(0, 1)
    expect(await waiting).toBeNull()
    const q = pane({ key: 'p2' })
    const w2 = waitShellResult(() => [q], 'p2')
    q.state = 'exited'
    expect(await w2).toBeNull()
  })
  it('resolves at once when a result is already there', async () => {
    expect(await waitShellResult(() => [pane({ shellLast: { code: 0, ms: 1, command: 'x' } })], 'p1')).toEqual({ code: 0 })
  })
})

describe('startCheckFromPhone', () => {
  function phone(over: Partial<PhoneRunDeps> = {}) {
    const launched: string[] = []
    let release: (v: { code: number }) => void = () => {}
    const deps: PhoneRunDeps = {
      integration: () => true,
      script: async () => 'npm run qa',
      head: async () => 'abc1234',
      report: async () => undefined,
      launch: (script) => (launched.push(script), 'pane-1'),
      wait: () => new Promise((r) => (release = r)),
      ...over,
    }
    return { deps, launched, finish: (code: number) => release({ code }) }
  }

  it('starts the check once and ignores a second request while it runs', async () => {
    const a = phone()
    expect(await startCheckFromPhone('/wt/p1', a.deps)).toEqual({ ok: true, started: true })
    await Promise.resolve()
    await Promise.resolve()
    expect(isCheckLive('/wt/p1')).toBe(true)
    const b = phone()
    expect(await startCheckFromPhone('/wt/p1', b.deps)).toEqual({ ok: false, error: 'running' })
    expect(a.launched).toEqual(['npm run qa'])
    expect(b.launched).toEqual([])
    a.finish(0)
    await Promise.resolve()
    await Promise.resolve()
    await Promise.resolve()
    expect(isCheckLive('/wt/p1')).toBe(false)
  })
  it('refuses when shell integration is off, the checklist is unavailable or has no script', async () => {
    expect(await startCheckFromPhone('/wt/p2', phone({ integration: () => false }).deps)).toEqual({ ok: false, error: 'integration' })
    expect(await startCheckFromPhone('/wt/p3', phone({ script: async () => null }).deps)).toEqual({ ok: false, error: 'unavailable' })
    expect(await startCheckFromPhone('/wt/p4', phone({ script: async () => '  ' }).deps)).toEqual({ ok: false, error: 'script' })
    expect(isCheckLive('/wt/p4')).toBe(false)
  })
})
