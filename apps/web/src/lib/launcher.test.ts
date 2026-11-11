import { describe, expect, it } from 'vitest'
import type { TerminalSession } from '@ccm/shared'
import { launchAgents, launchResumables, stepIndex } from './launcher'

const A = '11111111-1111-4111-8111-111111111111'
const B = '22222222-2222-4222-8222-222222222222'
const C = '33333333-3333-4333-8333-333333333333'

const term = (o: Partial<TerminalSession>): TerminalSession => ({ id: 't', title: '', status: 'idle', ...o }) as TerminalSession

describe('launcher helpers', () => {
  it('offers DeepSeek only when enabled', () => {
    expect(launchAgents(false)).toEqual(['claude', 'codex', 'shell'])
    expect(launchAgents(true)).toEqual(['claude', 'codex', 'deepseek', 'shell'])
  })

  it('wraps index steps', () => {
    expect(stepIndex(0, -1, 3)).toBe(2)
    expect(stepIndex(2, 1, 3)).toBe(0)
    expect(stepIndex(0, 1, 0)).toBe(-1)
  })

  it('lists ended sessions of the project, newest first, deduped, skipping open panes', () => {
    const list = [
      term({ id: '1', cwd: 'C:\\repo', claudeSessionId: A, endedAt: '2026-11-01T10:00:00Z' }),
      term({ id: '2', cwd: 'c:/repo/', claudeSessionId: B, endedAt: '2026-11-02T10:00:00Z' }),
      term({ id: '3', cwd: 'C:\\repo\\apps\\x', claudeSessionId: C, endedAt: '2026-11-03T10:00:00Z' }),
      term({ id: '4', cwd: 'C:\\repo', claudeSessionId: A, endedAt: '2026-10-01T10:00:00Z' }),
      term({ id: '5', cwd: 'C:\\other', claudeSessionId: C, endedAt: '2026-11-04T10:00:00Z' }),
      term({ id: '6', cwd: 'C:\\repo', claudeSessionId: C }),
      term({ id: '7', cwd: 'C:\\repo', provider: 'codex', codexSessionId: A, endedAt: '2026-11-05T10:00:00Z' }),
    ]
    expect(launchResumables(list, 'C:\\repo', 'claude', new Set()).map((r) => r.id)).toEqual([C, B, A])
    expect(launchResumables(list, 'C:\\repo', 'claude', new Set(['3'])).map((r) => r.id)).toEqual([B, A])
    expect(launchResumables(list, 'C:\\repo', 'codex', new Set()).map((r) => r.id)).toEqual([A])
    expect(launchResumables(list, 'C:\\repo', 'shell', new Set())).toEqual([])
    expect(launchResumables(list, undefined, 'claude', new Set())).toEqual([])
  })
})
