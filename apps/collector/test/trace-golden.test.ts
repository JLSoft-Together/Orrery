import { mkdir, writeFile } from 'node:fs/promises'
import path from 'node:path'
import { pathToFileURL } from 'node:url'
import { beforeAll, describe, expect, it } from 'vitest'
import { monitorScenario, type Trace } from './trace-scenario'

interface CompareLib {
  compareTraces: (a: unknown, b: unknown, options?: { strict?: boolean; limit?: number; ignore?: string[] }) => { diffs: string[]; steps: number }
  digest: (trace: unknown, options?: { strict?: boolean }) => Array<{ name: string; state: Record<string, Record<string, unknown>>; fired: unknown[]; direct: unknown[]; snapshots: unknown[] }>
}

const LIB = pathToFileURL(path.resolve(__dirname, '../../../scripts/trace-compare.mjs')).href
let lib: CompareLib

const WALL = { start: Date.parse('2026-11-10T10:00:00.000Z'), end: Date.parse('2026-11-10T10:05:00.000Z') }
const NOW = '2026-11-10T10:01:00.000Z'

function trace(steps: Array<{ name: string; frames: unknown[] }>, extra: Partial<Trace> = {}): Trace {
  return { format: 'ccm-trace/1', scenario: 't', source: 'node', wall: WALL, pids: {}, replace: [], ignore: [], steps: steps.map((s) => ({ sent: [], ...s })) as Trace['steps'], ...extra }
}

const terminal = (id: string, over: Record<string, unknown> = {}) => ({ id, title: 'demo', status: 'working', processId: Number(id.split(':')[0]), ...over })
const snapshot = (terminals: unknown[] = [], peerLinks: unknown[] = []) => ({
  seq: 0,
  events: [{ type: 'snapshot', payload: { protocolVersion: 1, collectorStartedAt: NOW, supportedClaudeVersion: '2.1.289', terminals, agents: [], activity: [], peerLinks } }],
})

beforeAll(async () => {
  lib = (await import(LIB)) as CompareLib
})

describe('trace-compare normalisation', () => {
  it('ignores seq, batch boundaries, key order and wall-clock times but keeps logical times', () => {
    const a = trace([
      { name: 'boot', frames: [snapshot()] },
      {
        name: 'one',
        frames: [
          { seq: 1, events: [{ type: 'terminal.created', payload: terminal('100:1', { startedAt: '2027-01-15T08:00:00.000Z', statusSince: NOW }) }] },
          { seq: 2, events: [{ type: 'terminal.updated', payload: { id: '100:1', status: 'idle', waitingFor: null } }] },
          { seq: 3, events: [{ type: 'activity', payload: { id: 'mhx1-0', at: NOW, kind: 'terminal.started', terminalId: '100:1' } }] },
        ],
      },
    ])
    const b = trace([
      { name: 'boot', frames: [snapshot()] },
      {
        name: 'one',
        frames: [
          {
            seq: 7,
            events: [
              { type: 'terminal.created', payload: { status: 'idle', title: 'demo', processId: 100, id: '100:1', statusSince: '2026-11-10T10:02:00.000Z', startedAt: '2027-01-15T08:00:00.000Z' } },
              { type: 'activity', payload: { kind: 'terminal.started', terminalId: '100:1', at: '2026-11-10T10:03:00.000Z', id: 'zzz-9' } },
            ],
          },
        ],
      },
    ])
    expect(lib.compareTraces(a, b).diffs).toEqual([])
    const changed = JSON.parse(JSON.stringify(b)) as Trace
    ;(changed.steps[1]!.frames[0]!.events[0]!.payload as { startedAt: string }).startedAt = '2027-01-15T08:00:01.000Z'
    expect(lib.compareTraces(a, changed).diffs).toEqual(['step 1 (one) state.terminals.100:1.startedAt: "2027-01-15T08:00:00.000Z" ≠ "2027-01-15T08:00:01.000Z"'])
  })

  it('maps placeholder pids to stable aliases and orients peer links after aliasing', () => {
    const link = (a: string, b: string, ab: number, ba: number, lastDir: string) => ({ id: `${a}>${b}`, kind: 'message', a, b, ab, ba, firstAt: '2027-01-15T08:00:00.000Z', lastAt: '2027-01-15T08:00:00.000Z', lastDir })
    const a = trace([{ name: 'boot', frames: [snapshot()] }, { name: 's', frames: [snapshot([terminal('300:5'), terminal('900:6')], [link('300:5', '900:6', 1, 0, 'ab')])] }], {
      pids: { '300': 'p0', '900': 'p1' },
    })
    const b = trace([{ name: 'boot', frames: [snapshot()] }, { name: 's', frames: [snapshot([terminal('7000:5'), terminal('40:6')], [link('40:6', '7000:5', 0, 1, 'ba')])] }], {
      pids: { '7000': 'p0', '40': 'p1' },
    })
    expect(lib.compareTraces(a, b).diffs).toEqual([])
  })

  it('reports fired events as a multiset and honours the ignore list', () => {
    const span = (outcome: string) => ({ type: 'tool.span', payload: { id: 't1', terminalId: '1:1', agentId: 'x', tool: 'Bash', start: '2027-01-15T08:00:00.000Z', outcome } })
    const resources = { type: 'resources', payload: { at: NOW, sessions: [] } }
    const a = trace([{ name: 'boot', frames: [snapshot()] }, { name: 's', frames: [{ seq: 1, events: [span('ok'), resources] }] }])
    const b = trace([{ name: 'boot', frames: [snapshot()] }, { name: 's', frames: [{ seq: 1, events: [span('interrupted')] }] }])
    const diffs = lib.compareTraces(a, b, { ignore: ['resources'] }).diffs
    expect(diffs).toHaveLength(2)
    expect(diffs[0]).toContain('only in a ×1')
    expect(diffs[1]).toContain('only in b ×1')
    expect(lib.compareTraces(a, b).diffs).toHaveLength(3)
  })

  it('compares every snapshot but the first boot one', () => {
    const a = trace([{ name: 'boot', frames: [snapshot([terminal('1:1')])] }, { name: 'resync', frames: [snapshot([terminal('1:1')])] }])
    const b = trace([{ name: 'boot', frames: [snapshot()] }, { name: 'resync', frames: [snapshot([terminal('1:1', { title: 'other' })])] }])
    const diffs = lib.compareTraces(a, b).diffs
    expect(diffs.some((d) => d.startsWith('step 0'))).toBe(true)
    expect(diffs.some((d) => d.includes('snapshots[0].terminals[1:1].title'))).toBe(true)
  })
})

describe('golden trace: monitor fixture scenario (Node vs Node)', () => {
  it('captures the same trace twice, strict and folded', async () => {
    const first = await monitorScenario()
    const second = await monitorScenario()
    const out = process.env.CCM_TRACE_OUT
    if (out) {
      await mkdir(out, { recursive: true })
      await writeFile(path.join(out, 'monitor-a.trace.json'), JSON.stringify(first))
      await writeFile(path.join(out, 'monitor-b.trace.json'), JSON.stringify(second))
    }
    expect(lib.compareTraces(first, second, { strict: true }).diffs).toEqual([])
    expect(lib.compareTraces(first, second).diffs).toEqual([])
    const steps = lib.digest(first)
    expect(steps.map((s) => s.name)).toEqual([
      'boot',
      'tool-result',
      'toolSpans.get',
      'alias-note',
      'nested-subagent',
      'nested-failed',
      'subagent-completed',
      'rename-waiting',
      'shell',
      'clear',
      'end',
      'stale-removal',
      'final',
    ])
    const at = (name: string) => steps.find((s) => s.name === name)!
    expect(at('toolSpans.get').direct).toHaveLength(3)
    expect(Object.values(at('nested-failed').state.agents!).map((a) => (a as { status: string }).status)).toContain('error')
    expect(Object.keys(at('stale-removal').state.terminals!)).toEqual([])
    expect(at('final').snapshots).toHaveLength(1)
    expect(JSON.stringify(first)).not.toContain('cc-msg')
  })
})
