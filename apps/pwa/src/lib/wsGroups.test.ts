import { describe, expect, it } from 'vitest'
import { DEFAULT_LOCAL_PREFS, normalizeLocalPrefs } from '../storage/localPrefs'
import { attentionCount, railShown } from './railModel'
import { monogram, parseWorkspace, toneOfProject } from './workspaceModel'

const pane = (key: string, extra: Record<string, unknown> = {}) => ({ key, kind: 'term', preset: 'claude', title: key, rail: 'working', sessionId: `s-${key}`, terminalId: `t-${key}`, cwd: `D:/Code/${key}`, ...extra })

const doc = (groups: unknown) => ({
  rev: 1,
  tabs: [{ key: 'tab1', name: 'Mix', panes: [pane('p1'), pane('p2', { rail: 'asked' }), pane('p3', { preset: 'codex' })], ...(groups === undefined ? {} : { groups }) }],
})

describe('workspace groups from tabs[].groups', () => {
  it('has no groups when the desktop sent none (older desktop, one project tab)', () => {
    expect(parseWorkspace(doc(undefined), [])[0]!.groups).toEqual([])
  })

  it('reads name, tone, folded and keeps pane order inside each group', () => {
    const [tab] = parseWorkspace(doc([{ project: 'D:/Code/api', name: 'API', tone: 3, folded: true, panes: ['p2', 'p1'] }, { project: 'D:/Code/web', panes: ['p3'] }]), [])
    expect(tab!.groups.map((g) => [g.name, g.tone, g.folded, g.panes.map((p) => p.key)])).toEqual([
      ['API', 3, true, ['p2', 'p1']],
      ['web', toneOfProject('D:/Code/web'), false, ['p3']],
    ])
  })

  it('wraps tone into 0 to 7, drops unknown or repeated panes and empty groups, collects leftovers', () => {
    const [tab] = parseWorkspace(doc([{ project: 'a', tone: 11, panes: ['p1', 'zz'] }, { project: 'b', panes: ['p1'] }, { project: 'c', panes: [] }, { nope: true }]), [])
    expect(tab!.groups.map((g) => g.panes.map((p) => p.key))).toEqual([['p1'], ['p2', 'p3']])
    expect(tab!.groups[0]!.tone).toBe(3)
    expect(tab!.groups[1]!.key).toBe('~rest')
    expect(tab!.panes).toHaveLength(3)
  })

  it('ignores a malformed groups field', () => {
    expect(parseWorkspace(doc('x'), [])[0]!.groups).toEqual([])
  })

  it('names tones and monograms stably', () => {
    expect(toneOfProject('orrery')).toBe(toneOfProject('orrery'))
    expect(toneOfProject('')).toBe(0)
    expect(monogram('web')).toBe('W')
    expect(monogram(null)).toBe('?')
  })
})

describe('iPad rail', () => {
  it('shows the rail only on wide screens, with a machine, on machine routes', () => {
    expect(railShown(true, 'monitor', 'm1')).toBe(true)
    expect(railShown(true, 'session', 'm1')).toBe(true)
    expect(railShown(true, 'workspace', 'm1')).toBe(true)
    expect(railShown(true, 'settings', 'm1')).toBe(true)
    expect(railShown(false, 'monitor', 'm1')).toBe(false)
    expect(railShown(true, 'monitor', null)).toBe(false)
    expect(railShown(true, 'home', 'm1')).toBe(false)
    expect(railShown(true, 'pair', 'm1')).toBe(false)
  })

  it('counts sessions that need you', () => {
    expect(attentionCount([{ rail: 'asked' }, { rail: 'failed' }, { rail: 'working' }, { rail: 'idle' }])).toBe(2)
  })

  it('remembers the collapsed rail, off by default', () => {
    expect(DEFAULT_LOCAL_PREFS.railCollapsed).toBe(false)
    expect(normalizeLocalPrefs({ railCollapsed: true }).railCollapsed).toBe(true)
    expect(normalizeLocalPrefs({ railCollapsed: 'yes' }).railCollapsed).toBe(false)
  })
})
