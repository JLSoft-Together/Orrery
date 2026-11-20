import { describe, expect, it } from 'vitest'
import type { ToolSpan } from '@ccm/shared'
import type { ChangeEntry } from './preview'
import { addPaths, editPaths, filterEntries, mergeTurn, parseTurns, serializeTurns, splitBySession, UNKNOWN_FILTER, validFilter, type TurnEdits } from './turnEdits'
import { stepEdges } from './turnEdges'

const T0 = Date.parse('2026-11-20T10:00:00.000Z')
let seq = 0
const span = (terminalId: string, path: string | undefined, at: number, access: 'edit' | 'read' = 'edit'): ToolSpan =>
  ({ id: `s${++seq}`, terminalId, agentId: `${terminalId}:main`, tool: access === 'edit' ? 'Edit' : 'Read', start: new Date(at).toISOString(), path, access }) as ToolSpan
const entry = (path: string, origPath?: string): ChangeEntry => ({ path, origPath, x: ' ', y: 'M', kind: origPath ? 'renamed' : 'modified', staged: false })
const turn = (terminalId: string, paths: string[], turnStart = T0): TurnEdits => ({ terminalId, name: terminalId.toUpperCase(), turnStart, paths: new Set(paths) })

describe('editPaths', () => {
  it('keeps edit spans with a path inside the turn window', () => {
    const spans = [span('a', '/r/one.ts', T0 + 1000), span('a', '/r/read.ts', T0 + 1000, 'read'), span('a', undefined, T0 + 1000), span('a', '/r/old.ts', T0 - 60_000), span('a', '/r/late.ts', T0 + 120_000)]
    expect([...editPaths(spans, T0, T0 + 60_000)]).toEqual(['/r/one.ts'])
    expect([...editPaths(spans, T0)].sort()).toEqual(['/r/late.ts', '/r/one.ts'])
  })
})

describe('addPaths / mergeTurn', () => {
  it('returns the same object when nothing is new and caps the set', () => {
    const t = turn('a', ['x'])
    expect(addPaths(t, ['x'])).toBe(t)
    expect([...addPaths(t, ['y', 'z'], 2).paths]).toEqual(['x', 'y'])
  })

  it('unions paths and keeps the earliest start of an unseen turn', () => {
    const merged = mergeTurn({ ...turn('a', ['x'], T0), turnEnd: T0 + 10 }, { ...turn('a', ['y'], T0 + 100), turnEnd: T0 + 200 })
    expect([...merged.paths].sort()).toEqual(['x', 'y'])
    expect(merged.turnStart).toBe(T0)
    expect(merged.turnEnd).toBe(T0 + 200)
  })
})

describe('splitBySession', () => {
  const root = '/r'
  const entries = [entry('a.ts'), entry('b.ts'), entry('bash.txt'), entry('both.ts'), entry('new.ts', 'old.ts')]
  const turns = [turn('a', ['/r/a.ts', '/r/both.ts']), turn('b', ['/r/b.ts', '/r/both.ts', '/r/old.ts']), turn('c', ['/elsewhere/x.ts'])]
  const split = splitBySession(root, entries, turns, (p) => p)

  it('puts each file under every session that edited it and the rest under unknown', () => {
    expect(split.sessions.map((s) => [s.terminalId, s.entries.map((e) => e.path)])).toEqual([
      ['a', ['a.ts', 'both.ts']],
      ['b', ['b.ts', 'both.ts', 'new.ts']],
    ])
    expect(split.unknown.map((e) => e.path)).toEqual(['bash.txt'])
    expect(split.owners.get('both.ts')).toEqual(['a', 'b'])
  })

  it('filters by session, unknown, or all', () => {
    expect(filterEntries(split, entries, 'a').map((e) => e.path)).toEqual(['a.ts', 'both.ts'])
    expect(filterEntries(split, entries, UNKNOWN_FILTER).map((e) => e.path)).toEqual(['bash.txt'])
    expect(filterEntries(split, entries, null)).toHaveLength(entries.length)
  })

  it('falls back to all for a session that is gone', () => {
    expect(validFilter(split, 'gone')).toBeNull()
    expect(validFilter(split, 'c')).toBeNull()
    expect(validFilter(split, 'a')).toBe('a')
    expect(validFilter(split, UNKNOWN_FILTER)).toBe(UNKNOWN_FILTER)
    expect(validFilter(splitBySession(root, [entry('a.ts')], turns, (p) => p), UNKNOWN_FILTER)).toBeNull()
  })
})

describe('persist', () => {
  it('round trips and drops junk', () => {
    const raw = serializeTurns([turn('a', ['x'])], [{ ...turn('b', ['y']), turnEnd: T0 + 5 }])
    const back = parseTurns(raw)
    expect(back.current.map((t) => [t.terminalId, [...t.paths]])).toEqual([['a', ['x']]])
    expect(back.last[0]?.turnEnd).toBe(T0 + 5)
    expect(parseTurns('{"v":1,"last":[{"terminalId":3}]}')).toEqual({ current: [], last: [] })
    expect(parseTurns('nope')).toEqual({ current: [], last: [] })
    expect(parseTurns(null)).toEqual({ current: [], last: [] })
  })
})

describe('stepEdges', () => {
  it('opens a turn on working and closes it on idle', () => {
    const first = stepEdges(new Map(), [{ id: 'a', status: 'idle' }], T0)
    const working = stepEdges(first, [{ id: 'a', status: 'working' }], T0 + 10)
    expect(working.get('a')).toEqual({ working: true, turnStart: T0 + 10 })
    const idle = stepEdges(working, [{ id: 'a', status: 'idle' }], T0 + 20)
    expect(idle.get('a')).toEqual({ working: false, turnStart: T0 + 10, idleSince: T0 + 20 })
    expect(stepEdges(idle, [], T0 + 30).size).toBe(0)
  })
})
