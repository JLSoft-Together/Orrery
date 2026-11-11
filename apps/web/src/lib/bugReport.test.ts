import { describe, expect, it } from 'vitest'
import type { BugReportItem } from './desktop'
import { SEEN_KEY, createUiErrorGate, diffCrashes, formatBytes, groupItems, isNoise, overLimit, readSeen, totalBytes, writeSeen } from './bugReport'

const items: BugReportItem[] = [
  { name: 'logs/pty.log', bytes: 2048, kind: 'log' },
  { name: 'report.txt', bytes: 100, kind: 'report' },
  { name: 'crashes/a.wer', bytes: 4096, kind: 'crash' },
  { name: 'dumps/x.dmp', bytes: 10 * 1024 ** 2, kind: 'dump' },
  { name: 'logs/shell.log', bytes: 1024, kind: 'log' },
]

describe('bugReport helpers', () => {
  it('formats sizes', () => {
    expect(formatBytes(12)).toBe('12 B')
    expect(formatBytes(2048)).toBe('2 KB')
    expect(formatBytes(5 * 1024 ** 2)).toBe('5 MB')
    expect(formatBytes(1.5 * 1024 ** 2)).toBe('1.5 MB')
    expect(formatBytes(-1)).toBe('—')
  })

  it('groups by kind in a fixed order and sums bytes', () => {
    const groups = groupItems(items)
    expect(groups.map((g) => g.kind)).toEqual(['report', 'log', 'crash', 'dump'])
    expect(groups[1]!.items.map((i) => i.name)).toEqual(['logs/pty.log', 'logs/shell.log'])
    expect(groups[1]!.bytes).toBe(3072)
    expect(groupItems([])).toEqual([])
  })

  it('totals with and without dumps', () => {
    expect(totalBytes(items, false)).toBe(2048 + 100 + 4096 + 1024)
    expect(totalBytes(items, true)).toBe(2048 + 100 + 4096 + 1024 + 10 * 1024 ** 2)
    expect(overLimit(51, 50)).toBe(true)
    expect(overLimit(50, 50)).toBe(false)
  })
})

describe('crash seen-set', () => {
  it('first run stores the set and stays quiet', () => {
    const d = diffCrashes(items, null)
    expect(d.first).toBe(true)
    expect(d.fresh).toEqual([])
    expect([...d.next].sort()).toEqual(['crashes/a.wer', 'dumps/x.dmp'])
  })

  it('reports only unseen crash and dump names', () => {
    const d = diffCrashes(items, new Set(['crashes/a.wer']))
    expect(d.first).toBe(false)
    expect(d.fresh).toEqual(['dumps/x.dmp'])
    expect(diffCrashes(items, d.next).fresh).toEqual([])
  })

  it('a grown events file counts as a new crash, a shrunk one does not', () => {
    const ev = (bytes: number) => [{ name: 'crashes/events.txt', bytes, kind: 'crash' as const }]
    const first = diffCrashes(ev(1000), null)
    expect(diffCrashes(ev(1000), first.next).fresh).toEqual([])
    const grown = diffCrashes(ev(1400), first.next)
    expect(grown.fresh).toEqual(['crashes/events.txt'])
    expect([...grown.next]).toEqual(['crashes/events.txt#1400'])
    expect(diffCrashes(ev(900), grown.next).fresh).toEqual([])
  })

  it('reads and writes the persisted set', () => {
    const store = new Map<string, string>()
    const storage = { getItem: (k: string) => store.get(k) ?? null, setItem: (k: string, v: string) => void store.set(k, v) }
    expect(readSeen(storage)).toBeNull()
    writeSeen(storage, new Set(['a', 'b']))
    expect(store.has(SEEN_KEY)).toBe(true)
    expect([...readSeen(storage)!]).toEqual(['a', 'b'])
    store.set(SEEN_KEY, '{bad')
    expect(readSeen(storage)).toBeNull()
    const broken = { getItem: () => { throw new Error('denied') } }
    expect(readSeen(broken)).toBeNull()
    expect(() => writeSeen({ setItem: () => { throw new Error('full') } }, new Set())).not.toThrow()
  })
})

describe('ui error throttling', () => {
  it('flags noise', () => {
    expect(isNoise('ResizeObserver loop completed with undelivered notifications.')).toBe(true)
    expect(isNoise('')).toBe(true)
    expect(isNoise('  ')).toBe(true)
    expect(isNoise('Script error.')).toBe(true)
    expect(isNoise('x', 'at chrome-extension://abc/content.js:1')).toBe(true)
    expect(isNoise('Cannot read properties of undefined')).toBe(false)
  })

  it('stays quiet in the first 3 s, then allows one toast per session', () => {
    let t = 1000
    const gate = createUiErrorGate(() => t)
    expect(gate('early')).toBe(false)
    t += 3000
    expect(gate('ResizeObserver loop limit exceeded')).toBe(false)
    expect(gate('real failure')).toBe(true)
    expect(gate('another failure')).toBe(false)
  })
})
