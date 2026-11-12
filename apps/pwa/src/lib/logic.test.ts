import QRCode from 'qrcode'
import { describe, expect, it } from 'vitest'
import { DEFAULT_LOCK_TIMEOUT, normalizeTimeout, shouldLock } from './lockPolicy'
import { groupMachines } from './machineGroups'
import {
  filterCounts,
  formatElapsed,
  groupByState,
  matchesFilter,
  parseActivity,
  parseSessions,
  sortSessions,
  type MonitorSession,
} from './monitorModel'
import { decodeFrame } from './qrDecode'
import { checkQrText, pairErrorKey } from './qrInput'
import { createAppI18n } from '../i18n'
import type { Machine } from '../remote/types'

function machine(id: string, name: string, pairedAt: number, revoked = false): Machine {
  return { id, name, mkey: new Uint8Array(32), pairedAt, policy: 'auto', revoked }
}

describe('groupMachines', () => {
  it('keeps one machine per name: ready first, then not revoked, then newest', () => {
    const list = [machine('a', 'PC', 100), machine('b', 'PC', 300), machine('c', 'PC', 200, true), machine('d', 'Mac', 50)]
    const byNewest = groupMachines(list, new Set())
    expect(byNewest.primary.map((m) => m.id)).toEqual(['b', 'd'])
    expect(byNewest.duplicates.map((m) => m.id)).toEqual(['c', 'a'])
    const withReady = groupMachines(list, new Set(['a']))
    expect(withReady.primary.map((m) => m.id)).toEqual(['a', 'd'])
  })

  it('prefers a live machine over a newer revoked one', () => {
    const list = [machine('new', 'PC', 500, true), machine('old', 'PC', 100)]
    expect(groupMachines(list, new Set()).primary.map((m) => m.id)).toEqual(['old'])
  })

  it('has no duplicates when names differ', () => {
    expect(groupMachines([machine('a', 'A', 1), machine('b', 'B', 2)], new Set()).duplicates).toEqual([])
  })
})

const snapshot = {
  terminals: [
    { id: 't1', title: 'idle one', status: 'idle', gitBranch: 'main', cwd: 'D:\\Code\\app', provider: 'claude' },
    { id: 't2', alias: 'Build', title: 'x', status: 'working', provider: 'codex', statusSince: '2026-01-01T10:00:00.000Z' },
    { id: 't3', title: 'ask', status: 'waiting', waitingFor: 'npm test' },
    { id: 't4', title: 'boom', status: 'error', provider: 'shell' },
    { id: 't5', title: 'finished', status: 'completed' },
  ],
  agents: [
    { id: 'm2', terminalId: 't2', role: 'main', currentTool: 'Edit', currentToolSummary: 'a.ts' },
    { id: 's1', terminalId: 't2', role: 'subagent', parentId: 'm2', status: 'working', description: 'first' },
    { id: 's2', terminalId: 't2', role: 'subagent', parentId: 's1', status: 'completed', name: 'nested' },
  ],
}

describe('monitor model', () => {
  const sessions = parseSessions(snapshot)

  it('parses status, provider, tool and nested sub-agents', () => {
    const t2 = sessions.find((s) => s.id === 't2')!
    expect(t2).toMatchObject({ title: 'Build', rail: 'working', provider: 'codex', tool: 'Edit', toolSummary: 'a.ts' })
    expect(t2.subagents.map((s) => [s.name, s.depth, s.rail])).toEqual([
      ['first', 1, 'working'],
      ['nested', 2, 'done'],
    ])
    expect(t2.since).toBe(Date.parse('2026-01-01T10:00:00.000Z'))
    expect(sessions.find((s) => s.id === 't4')?.rail).toBe('failed')
    expect(sessions.find((s) => s.id === 't1')).toMatchObject({ provider: 'claude', branch: 'main' })
  })

  it('returns nothing without a snapshot or terminal ids', () => {
    expect(parseSessions(null)).toEqual([])
    expect(parseSessions({ terminals: [{ title: 'no id' }, 5] })).toEqual([])
  })

  it('filters: running, waiting (asked + failed), idle (the rest)', () => {
    const ids = (f: Parameters<typeof matchesFilter>[0]) => sessions.filter((s) => matchesFilter(f, s)).map((s) => s.id)
    expect(ids('running')).toEqual(['t2'])
    expect(ids('waiting')).toEqual(['t3', 't4'])
    expect(ids('idle')).toEqual(['t1', 't5'])
    expect(ids('all')).toHaveLength(5)
    expect(filterCounts(sessions)).toEqual({ all: 5, running: 1, waiting: 2, idle: 2 })
  })

  it('sorts waiting first, then running, keeping source order inside a rank', () => {
    expect(sortSessions(sessions).map((s) => s.id)).toEqual(['t3', 't4', 't2', 't1', 't5'])
  })

  it('groups by state in a fixed order and drops empty groups', () => {
    expect(groupByState(sessions).map((g) => [g.rail, g.sessions.map((s) => s.id)])).toEqual([
      ['failed', ['t4']],
      ['asked', ['t3']],
      ['working', ['t2']],
      ['done', ['t5']],
      ['idle', ['t1']],
    ])
    expect(groupByState([] as MonitorSession[])).toEqual([])
  })

  it('formats elapsed time compactly in each language', () => {
    const en = createAppI18n('en').global.t
    const vi = createAppI18n('vi').global.t
    expect(formatElapsed(30_000, en)).toBe('<1m')
    expect(formatElapsed(5 * 60_000, en)).toBe('5m')
    expect(formatElapsed(3 * 3_600_000, en)).toBe('3h')
    expect(formatElapsed(50 * 3_600_000, en)).toBe('2d')
    expect(formatElapsed(30_000, vi)).toBe('<1p')
    expect(formatElapsed(5 * 60_000, vi)).toBe('5p')
    expect(formatElapsed(3 * 3_600_000, vi)).toBe('3g')
    expect(formatElapsed(50 * 3_600_000, vi)).toBe('2n')
  })

  it('reads the newest 12 activity lines of one terminal, newest first', () => {
    const rows = Array.from({ length: 20 }, (_, i) => ({
      id: `a${i}`,
      terminalId: i % 2 ? 't1' : 't2',
      kind: 'tool.started',
      at: new Date(Date.UTC(2026, 0, 1, 10, i)).toISOString(),
      data: { tool: `Tool${i}` },
    }))
    const out = parseActivity(rows, 't1')
    expect(out).toHaveLength(10)
    expect(out[0]).toMatchObject({ id: 'a19', detail: 'Tool19' })
    const many = parseActivity(
      Array.from({ length: 40 }, (_, i) => ({ id: `b${i}`, terminalId: 't1', kind: 'turn.completed', at: new Date(Date.UTC(2026, 0, 1, 10, i)).toISOString() })),
      't1',
    )
    expect(many).toHaveLength(12)
    expect(many[0]?.id).toBe('b39')
    expect(parseActivity([{ terminalId: 't1', kind: 'x', at: 'nope' }, 7], 't1')).toEqual([])
    expect(parseActivity('bad', 't1')).toEqual([])
  })
})

describe('qr input', () => {
  it('strips whitespace and accepts base64url', () => {
    expect(checkQrText('  eyJ2Ijoy\nNX0=  ')).toEqual({ ok: true, text: 'eyJ2IjoyNX0=' })
    expect(checkQrText('abc-DEF_123')).toMatchObject({ ok: true })
  })

  it('rejects empty, foreign characters and huge strings', () => {
    expect(checkQrText('   ')).toEqual({ ok: false, error: 'empty' })
    expect(checkQrText('orrery://pair?x=1')).toEqual({ ok: false, error: 'badChars' })
    expect(checkQrText('a'.repeat(5000))).toEqual({ ok: false, error: 'tooLong' })
  })

  it('maps pairing errors to user-facing kinds', () => {
    expect(pairErrorKey('denied')).toBe('denied')
    expect(pairErrorKey('expired')).toBe('expired')
    expect(pairErrorKey('badQr')).toBe('badCode')
    expect(pairErrorKey('noRoute')).toBe('noRoute')
    expect(pairErrorKey('noise')).toBe('failed')
  })

  it('decodes a real QR image frame', () => {
    const payload = 'eyJ2IjoyLCJtYWNoaW5lIjoiYWJjIn0'
    const qr = QRCode.create(payload, { errorCorrectionLevel: 'M' })
    const size = qr.modules.size
    const scale = 6
    const quiet = 4
    const dim = (size + quiet * 2) * scale
    const data = new Uint8ClampedArray(dim * dim * 4).fill(255)
    for (let y = 0; y < size; y++) {
      for (let x = 0; x < size; x++) {
        if (!qr.modules.get(x, y)) continue
        for (let dy = 0; dy < scale; dy++) {
          for (let dx = 0; dx < scale; dx++) {
            const px = ((y + quiet) * scale + dy) * dim + (x + quiet) * scale + dx
            data[px * 4] = 0
            data[px * 4 + 1] = 0
            data[px * 4 + 2] = 0
          }
        }
      }
    }
    expect(decodeFrame(data, dim, dim)).toBe(payload)
    expect(decodeFrame(new Uint8ClampedArray(64 * 64 * 4).fill(255), 64, 64)).toBeNull()
  })
})

describe('lock policy', () => {
  const hour = 3_600_000

  it('locks only when enabled and hidden for at least the timeout', () => {
    expect(shouldLock(true, hour, 0, hour)).toBe(true)
    expect(shouldLock(true, hour, 0, hour - 1)).toBe(false)
    expect(shouldLock(false, hour, 0, hour * 5)).toBe(false)
    expect(shouldLock(true, hour, null, hour * 5)).toBe(false)
    expect(shouldLock(true, 60_000, 1000, 61_000)).toBe(true)
  })

  it('falls back to one hour for unknown timeouts', () => {
    expect(normalizeTimeout(900_000)).toBe(900_000)
    expect(normalizeTimeout(123)).toBe(DEFAULT_LOCK_TIMEOUT)
    expect(normalizeTimeout('x')).toBe(DEFAULT_LOCK_TIMEOUT)
  })
})
