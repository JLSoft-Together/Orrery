import { describe, expect, it } from 'vitest'
import type { Agent } from '@ccm/shared'
import { endedToday, frozenSort, longestCommand, monitorGroups, runningCommands } from './monitorRows'

const live = { status: 'working' as const, provider: undefined }
const agent = (o: Partial<Agent> & { id: string }): Agent => ({ terminalId: 't1', role: 'main', status: 'working', ...o })

describe('runningCommands', () => {
  const terms: Record<string, { status: 'working' | 'stale' | 'waiting'; provider?: 'codex' }> = { t1: live, t2: { status: 'stale' }, t3: { status: 'waiting', provider: 'codex' } }
  const of = (id: string) => terms[id]

  it('keeps agents running a tool, longest first, unknown start last', () => {
    const rows = runningCommands(
      [
        agent({ id: 'a', currentTool: 'Bash', currentToolStartedAt: '2026-11-11T10:00:10.000Z' }),
        agent({ id: 'b', currentTool: 'Read' }),
        agent({ id: 'c', role: 'subagent', currentTool: 'Grep', currentToolSummary: 'rg foo', currentToolStartedAt: '2026-11-11T10:00:00.000Z' }),
        agent({ id: 'd', status: 'completed' as Agent['status'], currentTool: 'Bash' }),
        agent({ id: 'e' }),
        agent({ id: 'f', terminalId: 't2', currentTool: 'Bash' }),
      ],
      of,
    )
    expect(rows.map((r) => r.agentId)).toEqual(['c', 'a', 'b'])
    expect(rows[0]!.command).toBe('rg foo')
    expect(rows[2]!.command).toBe('Read')
    expect(rows[2]!.startedAt).toBeNull()
  })

  it('marks a main agent of a waiting session as waiting and picks the provider', () => {
    const [r] = runningCommands([agent({ id: 'x', terminalId: 't3', currentTool: 'Bash', currentToolSummary: 'rm -rf build' })], of)
    expect(r!.waiting).toBe(true)
    expect(r!.provider).toBe('codex')
  })

  it('falls back to the tool name without a summary', () => {
    const [r] = runningCommands([agent({ id: 'x', currentTool: 'Edit' })], of)
    expect(r!.command).toBe('Edit')
  })

  it('longestCommand prefers a row with a known start', () => {
    const rows = runningCommands([agent({ id: 'b', currentTool: 'Read' }), agent({ id: 'a', currentTool: 'Bash', currentToolStartedAt: '2026-11-11T10:00:00.000Z' })], of)
    expect(longestCommand(rows)?.agentId).toBe('a')
    expect(longestCommand([])).toBeUndefined()
  })
})

describe('monitorGroups', () => {
  const noon = new Date(2026, 10, 11, 12).getTime()
  it('drops sessions that ended before today', () => {
    const groups = monitorGroups(
      [
        { id: 'a', status: 'waiting' },
        { id: 'b', status: 'stale', endedAt: new Date(2026, 10, 11, 9).toISOString() },
        { id: 'c', status: 'stale', endedAt: new Date(2026, 10, 9, 9).toISOString() },
      ],
      noon,
    )
    expect(groups.map((g) => [g.key, g.ids])).toEqual([['needs', ['a']], ['ended', ['b']]])
    expect(endedToday({ status: 'stale' }, noon)).toBe(true)
  })
})

describe('frozenSort', () => {
  it('keeps the frozen order and puts unseen sessions last without NaN', () => {
    const out = frozenSort([{ id: 'n2' }, { id: 'b' }, { id: 'n1' }, { id: 'a' }], ['a', 'b'])
    expect(out.map((x) => x.id)).toEqual(['a', 'b', 'n1', 'n2'])
  })
})
