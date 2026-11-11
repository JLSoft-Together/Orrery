import { describe, expect, it } from 'vitest'
import type { ToolSpan } from '@ccm/shared'
import { AFTER_TURN_MS, buildTouchIndex, MAX_FILES, type TouchTerminal } from './agentTouches'

const NOW = Date.parse('2026-11-11T10:10:00.000Z')
const at = (offsetMs: number) => new Date(NOW + offsetMs).toISOString()
const span = (id: string, terminalId: string, path: string, access: 'edit' | 'read', offset = -1000): ToolSpan => ({
  id,
  terminalId,
  agentId: `${terminalId}:main`,
  tool: access === 'edit' ? 'Edit' : 'Read',
  start: at(offset),
  path,
  access,
})
const working = (name: string, provider: TouchTerminal['provider'] = 'claude'): TouchTerminal => ({ name, provider, working: true, turnStart: NOW - 60_000 })
const terms = (o: Record<string, TouchTerminal>) => new Map(Object.entries(o))

describe('buildTouchIndex', () => {
  it('marks edit and read per file, case-insensitive on Windows drives, and aggregates folders', () => {
    const idx = buildTouchIndex([span('1', 't1', 'F:/Work/App/src/A.ts', 'edit'), span('2', 't1', 'F:/work/app/src/b.ts', 'read')], terms({ t1: working('main') }), NOW)
    expect(idx.files.get('f:/work/app/src/a.ts')?.state).toBe('edit')
    expect(idx.files.get('f:/work/app/src/b.ts')?.state).toBe('read')
    expect(idx.dirs.get('f:/work/app/src')?.state).toBe('edit')
    expect(idx.dirs.get('f:/work')?.state).toBe('edit')
  })

  it('two agents editing one file is a conflict, an edit beats a read of the same agent', () => {
    const idx = buildTouchIndex(
      [span('1', 't1', '/p/a.ts', 'edit'), span('2', 't2', '/p/a.ts', 'edit'), span('3', 't1', '/p/a.ts', 'read', -500), span('4', 't3', '/p/a.ts', 'read')],
      terms({ t1: working('one'), t2: working('two', 'codex'), t3: working('three') }),
      NOW,
    )
    const f = idx.files.get('/p/a.ts')!
    expect(f.state).toBe('conflict')
    expect(f.editors).toBe(2)
    expect(f.touches.find((t) => t.terminalId === 't1')?.kind).toBe('edit')
    expect(idx.dirs.get('/p')?.state).toBe('conflict')
  })

  it('a read by one agent plus an edit by another is a plain edit', () => {
    const idx = buildTouchIndex([span('1', 't1', '/p/a.ts', 'read'), span('2', 't2', '/p/a.ts', 'edit')], terms({ t1: working('a'), t2: working('b') }), NOW)
    expect(idx.files.get('/p/a.ts')?.state).toBe('edit')
  })

  it('drops spans older than the current turn', () => {
    const idx = buildTouchIndex([span('1', 't1', '/p/old.ts', 'edit', -120_000), span('2', 't1', '/p/new.ts', 'edit')], terms({ t1: working('a') }), NOW)
    expect([...idx.files.keys()]).toEqual(['/p/new.ts'])
  })

  it('keeps marks for 2 minutes after the turn ends, then clears them', () => {
    const idle = (since: number): TouchTerminal => ({ name: 'a', provider: 'claude', working: false, turnStart: NOW - 90_000, idleSince: since })
    const s = [span('1', 't1', '/p/a.ts', 'edit', -30_000)]
    expect(buildTouchIndex(s, terms({ t1: idle(NOW - AFTER_TURN_MS + 1000) }), NOW).files.size).toBe(1)
    expect(buildTouchIndex(s, terms({ t1: idle(NOW - AFTER_TURN_MS - 1000) }), NOW).files.size).toBe(0)
    expect(buildTouchIndex(s, terms({ t1: { name: 'a', working: false } }), NOW).files.size).toBe(0)
  })

  it('ignores spans without a path, unknown terminals and unknown access', () => {
    const bare: ToolSpan = { id: 'x', terminalId: 't1', agentId: 'a', tool: 'Bash', start: at(-100) }
    const odd = { ...span('y', 't1', '/p/a.ts', 'edit'), access: 'delete' } as unknown as ToolSpan
    const idx = buildTouchIndex([bare, odd, span('z', 'gone', '/p/b.ts', 'edit')], terms({ t1: working('a') }), NOW)
    expect(idx.files.size).toBe(0)
  })

  it('caps the number of files', () => {
    const many = Array.from({ length: MAX_FILES + 50 }, (_, i) => span(String(i), 't1', `/p/f${i}.ts`, 'read'))
    expect(buildTouchIndex(many, terms({ t1: working('a') }), NOW).files.size).toBe(MAX_FILES)
  })
})
