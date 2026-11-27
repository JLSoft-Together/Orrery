import { describe, expect, it } from 'vitest'
import type { Pane, Tab } from '../../workspace/stores/workspace'
import { buildSummary, summaryHash, type RemotePaneFacts } from './remoteSummary'

const term = (key: string, extra: Partial<Pane> = {}): Pane => ({ key, preset: 'claude', cwd: 'C:\\work\\A', state: 'running', ...extra })

const facts = (over: Partial<RemotePaneFacts> = {}) => (): RemotePaneFacts => ({ title: 'Fix the bug', rail: 'working', busy: true, subagents: 0, ...over })

const tabs: Tab[] = [
  { key: 't1', root: { dir: 'row', ratio: 0.5, a: { pane: 'p1' }, b: { pane: 'p2' } }, focus: 'p1', project: 'C:\\work\\A', name: 'Main' },
  { key: 't2', root: { pane: 'p3' }, focus: 'p3', folded: true, zoom: 'p3' },
]

describe('buildSummary', () => {
  const panes: Pane[] = [
    term('p1', { sessionId: 'pty-1', terminalId: 'term-1', prefill: 'secret prompt', command: 'npm run secret', pendingCommand: 'rm -rf /' }),
    term('p2', { preset: 'shell', liveCwd: 'C:\\work\\A\\src', attention: 'done' }),
    { key: 'p3', kind: 'file', preset: 'shell', path: 'C:\\private\\keys\\token.txt', url: 'https://secret.example/x', state: 'running', dirty: true },
  ]

  it('follows the tab tree and keeps the shape of the contract', () => {
    const s = buildSummary({ tabs, panes, closedCount: 2, facts: facts() }, 7)
    expect(s.rev).toBe(7)
    expect(s.closedCount).toBe(2)
    expect(s.tabs.map((t) => t.key)).toEqual(['t1', 't2'])
    expect(s.tabs[0]).toMatchObject({ name: 'Main', project: 'C:\\work\\A', tree: { dir: 'row', ratio: 0.5, a: { pane: 'p1' }, b: { pane: 'p2' } } })
    expect(s.tabs[0]!.panes.map((p) => p.key)).toEqual(['p1', 'p2'])
    expect(s.tabs[0]!.panes[0]).toEqual({
      key: 'p1',
      kind: 'term',
      preset: 'claude',
      agent: 'claude',
      title: 'Fix the bug',
      cwd: 'C:\\work\\A',
      state: 'running',
      rail: 'working',
      subagents: 0,
      sessionId: 'pty-1',
      terminalId: 'term-1',
      dirty: false,
      busy: true,
    })
    expect(s.tabs[1]).toMatchObject({ zoom: 'p3', folded: true })
    expect(s.tabs[0]!.panes[1]).toMatchObject({ cwd: 'C:\\work\\A\\src', attention: 'done' })
  })

  it('never carries urls, file paths, prefill, commands or env', () => {
    const json = JSON.stringify(buildSummary({ tabs, panes, closedCount: 0, facts: facts({ title: 'x' }) }, 1))
    for (const leak of ['secret prompt', 'npm run secret', 'rm -rf', 'private', 'keys', 'secret.example', 'prefill', 'pendingCommand', 'env']) expect(json).not.toContain(leak)
    expect(json).toContain('"title":"token.txt"')
  })

  it('names the wait reason only for a pane that is asking', () => {
    const asked = buildSummary({ tabs, panes, closedCount: 0, facts: facts({ rail: 'asked', waitingFor: 'permission prompt' }) }, 1)
    expect(asked.tabs[0]!.panes[0]!.waitText).toBe('permission')
    const idle = buildSummary({ tabs, panes, closedCount: 0, facts: facts({ rail: 'idle', waitingFor: 'permission prompt' }) }, 1)
    expect(idle.tabs[0]!.panes[0]).not.toHaveProperty('waitText')
  })

  it('carries the inbox reason per pane, flags the snapshot and hashes the change', () => {
    const plain = buildSummary({ tabs, panes, closedCount: 0, facts: facts() }, 1)
    expect(plain).not.toHaveProperty('inbox')
    expect(plain.tabs[0]!.panes[0]).not.toHaveProperty('inbox')
    const inbox = (p: Pane) => (p.key === 'p1' ? { reason: 'clash' as const, since: '2026-11-20T01:00:00Z', file: `src/${'a'.repeat(200)}.ts`, peer: 'B' } : undefined)
    const s = buildSummary({ tabs, panes, closedCount: 0, facts: facts(), inbox }, 1)
    expect(s.inbox).toBe(true)
    expect(s.tabs[0]!.panes[0]!.inbox).toMatchObject({ reason: 'clash', since: '2026-11-20T01:00:00Z', peer: 'B' })
    expect(s.tabs[0]!.panes[0]!.inbox!.file!.length).toBe(120)
    expect(s.tabs[0]!.panes[1]).not.toHaveProperty('inbox')
    const empty = buildSummary({ tabs, panes, closedCount: 0, facts: facts(), inbox: () => undefined }, 1)
    expect(summaryHash(empty)).not.toBe(summaryHash(s))
    expect(summaryHash(empty)).not.toBe(summaryHash(plain))
  })

  it('skips leaves whose pane is gone and clips long titles', () => {
    const s = buildSummary({ tabs, panes: [panes[0]!], closedCount: 0, facts: facts({ title: 'x'.repeat(500) }) }, 1)
    expect(s.tabs[0]!.panes.map((p) => p.key)).toEqual(['p1'])
    expect(s.tabs[0]!.panes[0]!.title).toHaveLength(120)
  })

  it('keeps the tree flat for old apps and lists project groups beside it', () => {
    const grouped: Tab[] = [
      {
        key: 'tg',
        root: { dir: 'row', ratio: 0.6, a: { group: 'C:\\work\\A', node: { dir: 'col', ratio: 0.5, a: { pane: 'p1' }, b: { pane: 'p2' } } }, b: { group: 'C:\\work\\B', name: '  API\u0007 ', tone: 3, folded: true, node: { pane: 'p3' } } },
        focus: 'p1',
        project: 'C:\\work\\A',
      },
    ]
    const s = buildSummary({ tabs: grouped, panes, closedCount: 0, facts: facts() }, 1)
    const tab = s.tabs[0]!
    expect(tab.tree).toEqual({ dir: 'row', ratio: 0.6, a: { dir: 'col', ratio: 0.5, a: { pane: 'p1' }, b: { pane: 'p2' } }, b: { pane: 'p3' } })
    expect(tab.groups).toEqual([
      { project: 'C:\\work\\A', panes: ['p1', 'p2'] },
      { project: 'C:\\work\\B', name: 'API', tone: 3, folded: true, panes: ['p3'] },
    ])
    expect(tab.panes.map((p) => p.key)).toEqual(['p1', 'p2', 'p3'])
    expect(buildSummary({ tabs, panes, closedCount: 0, facts: facts() }, 1).tabs.every((t) => !('groups' in t))).toBe(true)
  })

  it('hashes the content, not the revision, and changes with it', () => {
    const a = buildSummary({ tabs, panes, closedCount: 0, facts: facts() }, 1)
    const b = buildSummary({ tabs, panes, closedCount: 0, facts: facts() }, 99)
    expect(summaryHash(a)).toBe(summaryHash(b))
    const c = buildSummary({ tabs, panes, closedCount: 0, facts: facts({ rail: 'idle' }) }, 1)
    expect(summaryHash(c)).not.toBe(summaryHash(a))
    const d = buildSummary({ tabs, panes, closedCount: 1, facts: facts() }, 1)
    expect(summaryHash(d)).not.toBe(summaryHash(a))
  })
})

describe('buildSummary question and snoozed', () => {
  const p1 = term('p1', { terminalId: 'term-1' })
  const p2 = term('p2', { terminalId: 'term-2' })
  const tab: Tab[] = [{ key: 't1', root: { dir: 'row', ratio: 0.5, a: { pane: 'p1' }, b: { pane: 'p2' } }, focus: 'p1' }]

  it('adds a masked question for a waiting pane without a menu and the snoozed flag', () => {
    const waiting = facts({ rail: 'asked', waitingFor: 'Deploy with token=abc123def456ghi now?' })
    const s = buildSummary({ tabs: tab, panes: [p1, p2], closedCount: 0, facts: waiting, snoozed: (p) => p.key === 'p2' }, 1)
    const [a, b] = s.tabs[0]!.panes
    expect(a!.question).toBe('Deploy with ••• now?')
    expect(a!.snoozed).toBeUndefined()
    expect(b!.snoozed).toBe(true)
    expect(summaryHash(s)).not.toBe(summaryHash(buildSummary({ tabs: tab, panes: [p1, p2], closedCount: 0, facts: waiting }, 1)))
  })

  it('caps the question at 300 characters and drops it with a menu, for a working pane or an empty reason', () => {
    const long = buildSummary({ tabs: tab, panes: [p1, p2], closedCount: 0, facts: facts({ rail: 'asked', waitingFor: 'x'.repeat(500) }) }, 1)
    expect(long.tabs[0]!.panes[0]!.question).toHaveLength(300)
    const menu = { sig: 'abc', kind: 'permission', agent: 'claude', question: 'q', items: [], numbered: true, confident: true, since: 1 } as never
    const withMenu = buildSummary({ tabs: tab, panes: [p1, p2], closedCount: 0, facts: facts({ rail: 'asked', waitingFor: 'why' }), menu: () => menu }, 1)
    expect(withMenu.tabs[0]!.panes[0]!.question).toBeUndefined()
    const working = buildSummary({ tabs: tab, panes: [p1, p2], closedCount: 0, facts: facts({ rail: 'working', waitingFor: 'why' }) }, 1)
    expect(working.tabs[0]!.panes[0]!.question).toBeUndefined()
    const blank = buildSummary({ tabs: tab, panes: [p1, p2], closedCount: 0, facts: facts({ rail: 'asked', waitingFor: '   ' }) }, 1)
    expect(blank.tabs[0]!.panes[0]!.question).toBeUndefined()
  })
})
