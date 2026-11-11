import { describe, expect, it } from 'vitest'
import type { Pane, Tab } from '../stores/workspace'
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

  it('skips leaves whose pane is gone and clips long titles', () => {
    const s = buildSummary({ tabs, panes: [panes[0]!], closedCount: 0, facts: facts({ title: 'x'.repeat(500) }) }, 1)
    expect(s.tabs[0]!.panes.map((p) => p.key)).toEqual(['p1'])
    expect(s.tabs[0]!.panes[0]!.title).toHaveLength(120)
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
