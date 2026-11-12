import { describe, expect, it } from 'vitest'
import type { MonitorSession } from './monitorModel'
import { diffCounts, parseChanges, parseDiff, parseDir, parseLog, relativePath, samePath, sizeText } from './viewerModel'
import { viewerCall, ViewerError } from './viewerApi'
import { findPane, paneRail, parseWorkspace, previewLines } from './workspaceModel'
import { RemoteCommandError } from '../remote/types'

const session = (over: Partial<MonitorSession>): MonitorSession => ({ id: 't1', title: 'x', rail: 'working', provider: 'claude', branch: null, cwd: null, tool: null, toolSummary: null, waitingFor: null, since: 1000, subagents: [], ...over })

const doc = {
  rev: 3,
  tabs: [
    {
      key: 'tab1',
      project: 'D:\\code\\orrery',
      panes: [
        { key: 'p1', kind: 'term', preset: 'claude', title: 'Dựng UI', rail: 'working', sessionId: 's1', terminalId: 't1', subagents: 2, cwd: 'D:\\code\\orrery', branch: 'main' },
        { key: 'p2', kind: 'term', preset: 'codex', title: 'Test', rail: 'asked', waitText: 'Chạy npm test?', sessionId: 's2', terminalId: 't2' },
        { key: 'p3', kind: 'file', title: 'a.ts', rail: 'idle' },
      ],
    },
    { key: 'tab2', name: 'Ads', panes: [] },
    { nokey: true },
  ],
}

describe('workspace model', () => {
  it('parses tabs and panes, names tabs by name or project folder', () => {
    const tabs = parseWorkspace(doc, [session({ tool: 'Bash', toolSummary: 'npm test' })])
    expect(tabs.map((t) => t.name)).toEqual(['orrery', 'Ads'])
    const [p1, p2, p3] = tabs[0]!.panes
    expect(p1).toMatchObject({ key: 'p1', provider: 'claude', rail: 'working', tool: 'Bash', canAttach: true, subagents: 2, branch: 'main', since: 1000 })
    expect(p2).toMatchObject({ provider: 'codex', rail: 'asked', asked: true, waitText: 'Chạy npm test?' })
    expect(p3).toMatchObject({ isTerm: false, canAttach: false, provider: 'claude' })
  })

  it('maps pane rails to the status vocabulary', () => {
    expect(paneRail('subagents')).toBe('working')
    expect(paneRail('failed')).toBe('failed')
    expect(paneRail('parked')).toBe('idle')
    expect(paneRail(undefined)).toBe('idle')
  })

  it('finds a pane in any tab and handles a missing doc', () => {
    const tabs = parseWorkspace(doc, [])
    expect(findPane(tabs, 'p2')?.tab.key).toBe('tab1')
    expect(findPane(tabs, 'zz')).toBeNull()
    expect(parseWorkspace(null, [])).toEqual([])
  })

  it('previews the running tool and the wait text', () => {
    const [p1] = parseWorkspace(doc, [session({ tool: 'Bash', toolSummary: 'npm test' })])[0]!.panes
    expect(previewLines(p1!)).toEqual(['> Bash npm test', '+2'])
  })
})

describe('viewer model', () => {
  const text = '--- a/x\n+++ b/x\n@@ -12,3 +12,4 @@ class A\n ctx\n-old\n+new\n+newer\n ctx2\n@@ -40,1 +43,1 @@\n+z\n\\ No newline at end of file\n'

  it('parses a unified diff with line numbers per side', () => {
    const lines = parseDiff(text)
    expect(lines.map((l) => l.kind)).toEqual(['hunk', 'ctx', 'del', 'add', 'add', 'ctx', 'hunk', 'add'])
    expect(lines[1]).toMatchObject({ oldNo: 12, newNo: 12, text: 'ctx' })
    expect(lines[2]).toMatchObject({ oldNo: 13, newNo: null, text: 'old' })
    expect(lines[4]).toMatchObject({ oldNo: null, newNo: 14, text: 'newer' })
    expect(lines[7]).toMatchObject({ newNo: 43 })
    expect(diffCounts(lines)).toEqual({ added: 3, removed: 1 })
  })

  it('parses directory entries folders first and skips out-of-root', () => {
    const rows = parseDir([{ name: 'b.ts', path: '/r/b.ts' }, { name: 'src', path: '/r/src', directory: true }, { name: 'A.ts', path: '/r/A.ts' }, { name: 'x', path: '/x', outOfRoot: true }])
    expect(rows.map((r) => r.name)).toEqual(['src', 'A.ts', 'b.ts'])
  })

  it('parses changes and log pages', () => {
    const changes = parseChanges({ branch: 'main', entries: [{ path: 'a.ts', kind: 'untracked' }, { path: 'b.ts', kind: 'modified', added: 3, removed: 1 }, { nopath: 1 }] })
    expect(changes.entries.map((e) => e.mark)).toEqual(['?', 'M'])
    expect(changes.entries[1]).toMatchObject({ added: 3, removed: 1 })
    const log = parseLog({ commits: [{ hash: 'abcdef1234', author: 'A', time: 5, subject: 's', refs: 'HEAD, main' }], more: true })
    expect(log.commits[0]).toMatchObject({ short: 'abcdef1', refs: ['HEAD', 'main'] })
    expect(log.more).toBe(true)
  })

  it('formats sizes and paths', () => {
    expect(sizeText(900)).toBe('900 B')
    expect(sizeText(2048)).toBe('2.0 KB')
    expect(relativePath('D:\\code\\orrery', 'D:\\code\\orrery\\src\\a.ts')).toBe('src/a.ts')
    expect(samePath('D:\\Code\\X\\', 'd:/code/x')).toBe(true)
  })

  it('viewer calls separate data, pending confirm, cancel and desktop errors', async () => {
    expect(await viewerCall(async () => ({ data: [1] }), 'fs.list')).toEqual({ kind: 'data', data: [1] })
    expect(await viewerCall(async () => ({ confirm: 'c1' }), 'git.discard')).toEqual({ kind: 'pending' })
    expect(await viewerCall(async () => ({ cancelled: true }), 'git.discard')).toEqual({ kind: 'cancelled' })
    await expect(viewerCall(async () => ({ data: { ok: false, error: 'conflict' } }), 'fs.write')).rejects.toMatchObject({ code: 'conflict' })
    await expect(viewerCall(async () => ({}), 'fs.read')).rejects.toMatchObject({ code: 'denied' })
    await expect(
      viewerCall(async () => {
        throw new RemoteCommandError('timeout')
      }, 'fs.read'),
    ).rejects.toBeInstanceOf(ViewerError)
  })
})
