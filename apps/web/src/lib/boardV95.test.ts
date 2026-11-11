import { describe, expect, it } from 'vitest'
import type { ToolSpan } from '@ccm/shared'
import { clockLabel, columnTemplate, doneRow, fileAction, isStrip, laneSessions, parseGroupBy, sortDone, splitDone } from './boardV95'

const span = (o: Partial<ToolSpan>): ToolSpan => ({ id: 's', terminalId: 't', agentId: 'a', tool: 'Edit', start: '2026-11-11T10:00:00Z', ...o })

describe('fileAction', () => {
  it('returns the basename of the newest open span with a path', () => {
    const r = fileAction(
      [
        span({ id: '1', path: 'src/old.ts', access: 'edit', start: '2026-11-11T10:00:00Z' }),
        span({ id: '2', path: 'C:\\w\\src\\new.ts', access: 'read', start: '2026-11-11T10:05:00Z' }),
        span({ id: '3', path: 'x/closed.ts', access: 'edit', start: '2026-11-11T10:09:00Z', end: '2026-11-11T10:09:01Z' }),
        span({ id: '4', agentId: 'other', path: 'x/other.ts', access: 'edit', start: '2026-11-11T10:10:00Z' }),
      ],
      'a',
    )
    expect(r).toEqual({ access: 'read', file: 'new.ts' })
  })
  it('is null without a path, access or agent', () => {
    expect(fileAction([span({ access: 'edit' })], 'a')).toBeNull()
    expect(fileAction([span({ path: 'a.ts' })], 'a')).toBeNull()
    expect(fileAction([span({ path: 'a.ts', access: 'edit' })], undefined)).toBeNull()
  })
})

describe('columns', () => {
  it('collapses only empty columns that are not expanded', () => {
    expect(isStrip(0, false)).toBe(true)
    expect(isStrip(0, true)).toBe(false)
    expect(isStrip(2, false)).toBe(false)
  })
  it('builds the grid template', () => {
    expect(columnTemplate([false, true, false, true])).toBe('minmax(0, 1fr) 40px minmax(0, 1fr) 40px')
  })
})

describe('done timeline', () => {
  it('pads the clock', () => {
    expect(clockLabel(new Date(2026, 10, 11, 9, 5).getTime())).toBe('09:05')
  })
  it('builds a row from endedAt, falling back to lastActivityAt, with files only when known', () => {
    const a = doneRow({ id: 'a', endedAt: new Date(2026, 10, 11, 14, 2).toISOString() }, 'T', 'p')
    expect(a).toMatchObject({ time: '14:02', files: null })
    const b = doneRow({ id: 'b', lastActivityAt: new Date(2026, 10, 11, 8, 0).toISOString(), diff: { files: 3, insertions: 1, deletions: 0, untracked: 0, at: '' } }, 'T', 'p')
    expect(b?.files).toBe(3)
    expect(doneRow({ id: 'c' }, 'T', 'p')).toBeNull()
    expect(doneRow({ id: 'd', endedAt: 'nope' }, 'T', 'p')).toBeNull()
  })
  it('sorts newest first and splits after three', () => {
    const rows = [1, 2, 3, 4, 5].map((n) => ({ id: `r${n}`, at: n, time: '', title: '', project: '', files: null }))
    const sorted = sortDone(rows)
    expect(sorted.map((r) => r.id)).toEqual(['r5', 'r4', 'r3', 'r2', 'r1'])
    expect(splitDone(sorted, false)).toMatchObject({ more: 2 })
    expect(splitDone(sorted, false).shown).toHaveLength(3)
    expect(splitDone(sorted, true)).toMatchObject({ more: 0 })
    expect(splitDone(sorted.slice(0, 3), false).more).toBe(0)
  })
})

describe('parseGroupBy', () => {
  it('falls back to status', () => {
    expect(parseGroupBy('project')).toBe('project')
    expect(parseGroupBy('agent')).toBe('agent')
    expect(parseGroupBy('nope')).toBe('status')
    expect(parseGroupBy(null)).toBe('status')
  })
})

describe('laneSessions', () => {
  const rows = [
    { id: 'a', status: 'idle', cwd: '/w/app', lastActivityAt: '2026-11-11T10:00:00Z' },
    { id: 'b', status: 'working', cwd: '/w/api', provider: 'codex', lastActivityAt: '2026-11-11T11:00:00Z' },
    { id: 'c', status: 'waiting', cwd: '/w/app', lastActivityAt: '2026-11-11T09:00:00Z' },
    { id: 'd', status: 'stale', cwd: '/w/app', lastActivityAt: '2026-11-11T08:00:00Z' },
    { id: 'e', status: 'working', cwd: '/w/app', lastActivityAt: '2026-11-11T12:00:00Z' },
  ] as const

  it('groups by project: needs-owner first, cards sorted needs then running then idle, ended apart', () => {
    const lanes = laneSessions(rows, 'project')
    expect(lanes.map((l) => l.name)).toEqual(['app', 'api'])
    expect(lanes[0]!.active).toEqual(['c', 'e', 'a'])
    expect(lanes[0]!.ended).toEqual(['d'])
    expect(lanes[0]!.needs).toBe(1)
  })
  it('orders projects without needs by latest activity', () => {
    const lanes = laneSessions(rows.filter((r) => r.id !== 'c'), 'project')
    expect(lanes.map((l) => l.name)).toEqual(['app', 'api'])
    expect(laneSessions(rows.filter((r) => r.id === 'b' || r.id === 'a'), 'project').map((l) => l.name)).toEqual(['api', 'app'])
  })
  it('groups by agent in provider order and defaults to claude', () => {
    const lanes = laneSessions(rows, 'agent')
    expect(lanes.map((l) => l.provider)).toEqual(['claude', 'codex'])
    expect(lanes[1]!.active).toEqual(['b'])
    expect(lanes[0]!.active).toEqual(['c', 'e', 'a'])
  })
  it('keeps sessions without cwd in a nameless project lane', () => {
    const lanes = laneSessions([{ id: 'x', status: 'idle' }], 'project')
    expect(lanes).toHaveLength(1)
    expect(lanes[0]!.name).toBe('')
  })
})
