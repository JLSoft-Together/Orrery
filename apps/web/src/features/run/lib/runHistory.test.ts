import { describe, expect, it } from 'vitest'
import type { RunHistoryEntry, RunLogLine } from '@ccm/shared'
import { compareRuns, defaultCompare, exitTone, formatDuration, normalizeLine, COMPARE_MAX_DIFF } from './runHistory'

const entry = (over: Partial<RunHistoryEntry>): RunHistoryEntry => ({
  id: 'a',
  project: '/p',
  target: 'app',
  config: 'c',
  device: null,
  start: 1000,
  end: 4000,
  exitCode: 0,
  lines: 0,
  bytes: 0,
  pinned: false,
  truncated: false,
  ...over,
})

const lines = (...t: string[]): RunLogLine[] => t.map((text, i) => ({ stream: 'app', text, at: i }))

describe('normalizeLine', () => {
  it('drops timestamps, pids, temp paths and long hex', () => {
    expect(normalizeLine('2026-11-23T09:20:11.123Z build started')).toBe('<ts> build started')
    expect(normalizeLine('11-23 09:20:11.123  1234  5678 I App: hi')).toBe('<ts> 1234 5678 I App: hi')
    expect(normalizeLine('[09:20:11] pid 4242 ready')).toBe('[<ts>] pid <pid> ready')
    expect(normalizeLine('E/App(12345): boom')).toBe('E/App(<pid>): boom')
    expect(normalizeLine('writing C:\\Users\\x\\AppData\\Local\\Temp\\build_ab12\\out.txt now')).toBe('writing <tmp> now')
    expect(normalizeLine('cache /tmp/flutter_x1/y done')).toBe('cache <tmp> done')
    expect(normalizeLine('commit 9f3a1c0b7d2e applied')).toBe('commit <hex> applied')
    expect(normalizeLine('Built in 12.3s')).toBe('Built in <dur>')
  })

  it('keeps meaningful differences', () => {
    expect(normalizeLine('error: undefined name foo')).not.toBe(normalizeLine('error: undefined name bar'))
    expect(normalizeLine('  a   b  ')).toBe('a b')
  })
})

describe('compareRuns', () => {
  it('lists lines present on one side only, ignoring noise', () => {
    const a = lines('10:00:01 start', 'error: foo', 'pid 11 ok', 'same')
    const b = lines('10:05:09 start', 'pid 99 ok', 'same', 'warn: new')
    const r = compareRuns(entry({ exitCode: 1, start: 0, end: 5000 }), entry({ exitCode: 0, start: 0, end: 3000 }), a, b)
    expect(r.onlyOld.map((d) => d.text)).toEqual(['error: foo'])
    expect(r.onlyNew.map((d) => d.text)).toEqual(['warn: new'])
    expect(r.onlyOld[0]!.index).toBe(1)
    expect([r.oldCode, r.newCode, r.oldMs, r.newMs]).toEqual([1, 0, 5000, 3000])
  })

  it('counts duplicates as a multiset', () => {
    const r = compareRuns(entry({}), entry({}), lines('x', 'x', 'x'), lines('x'))
    expect(r.totalOnlyOld).toBe(2)
    expect(r.totalOnlyNew).toBe(0)
  })

  it('caps the list at 500 but keeps the total', () => {
    const many = Array.from({ length: 800 }, (_, i) => `unique line ${i}x`)
    const r = compareRuns(entry({}), entry({}), lines(...many), lines())
    expect(r.onlyOld.length).toBe(COMPARE_MAX_DIFF)
    expect(r.totalOnlyOld).toBe(800)
  })
})

describe('helpers', () => {
  it('formats durations', () => {
    expect(formatDuration(null)).toBe('')
    expect(formatDuration(420)).toBe('420 ms')
    expect(formatDuration(3200)).toBe('3.2 s')
    expect(formatDuration(125000)).toBe('2 m 05 s')
  })

  it('tones the exit code', () => {
    expect(exitTone({ exitCode: 0, end: 1 })).toBe('ok')
    expect(exitTone({ exitCode: 2, end: 1 })).toBe('error')
    expect(exitTone({ exitCode: null, end: 1 })).toBe('unknown')
    expect(exitTone({ exitCode: null, end: null })).toBe('unknown')
  })

  it('picks the pinned run as baseline against the newest run', () => {
    const list = [entry({ id: 'n3' }), entry({ id: 'n2' }), entry({ id: 'n1', pinned: true }), entry({ id: 'n0' })]
    expect(defaultCompare(list)).toEqual({ baseline: 'n1', latest: 'n3' })
    expect(defaultCompare([list[0]!, entry({ id: 'x', end: null })])).toBeNull()
    expect(defaultCompare([list[0]!, list[1]!])).toEqual({ baseline: 'n2', latest: 'n3' })
  })
})
