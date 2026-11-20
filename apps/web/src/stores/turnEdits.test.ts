import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { nextTick } from 'vue'
import { createPinia, setActivePinia } from 'pinia'
import type { TerminalSession, ToolSpan } from '@ccm/shared'
import { useConnectionStore } from './connection'
import { useTerminalsStore } from './terminals'
import { useToolSpansStore } from './toolSpans'
import { useTurnEditsStore } from './turnEdits'

const term = (id: string, status: TerminalSession['status']): TerminalSession => ({ id, title: id, status, startedAt: '2026-11-20T09:00:00.000Z' }) as TerminalSession
let seq = 0
const edit = (terminalId: string, path: string, access: 'edit' | 'read' = 'edit'): ToolSpan => ({ id: `s${++seq}`, terminalId, agentId: `${terminalId}:main`, tool: 'Edit', start: new Date().toISOString(), path, access })

async function status(list: [string, TerminalSession['status']][]): Promise<void> {
  useTerminalsStore().replaceAll(list.map(([id, s]) => term(id, s)))
  await nextTick()
  await nextTick()
}

describe('turnEdits store', () => {
  beforeEach(() => {
    sessionStorage.clear()
    setActivePinia(createPinia())
    useConnectionStore().hasData = true
    vi.useFakeTimers({ toFake: ['Date', 'setTimeout', 'clearTimeout'] })
    vi.setSystemTime(Date.parse('2026-11-20T10:00:00.000Z'))
  })
  afterEach(() => vi.useRealTimers())

  it('keeps the finished turn past AFTER_TURN_MS until marked seen', async () => {
    await status([['a', 'idle']])
    const store = useTurnEditsStore()
    await status([['a', 'working']])
    vi.advanceTimersByTime(1000)
    useToolSpansStore().upsert(edit('a', '/r/a.ts'))
    useToolSpansStore().upsert(edit('a', '/r/read.ts', 'read'))
    await nextTick()
    expect(store.countOf('a')).toBe(1)
    vi.advanceTimersByTime(1000)
    await status([['a', 'idle']])
    expect(store.last.get('a')?.paths).toEqual(new Set(['/r/a.ts']))
    vi.advanceTimersByTime(10 * 60_000)
    await nextTick()
    expect(store.countOf('a')).toBe(1)
    store.markSeen('a')
    expect(store.countOf('a')).toBe(0)
  })

  it('merges an unseen turn into the next one and saves to sessionStorage', async () => {
    await status([['a', 'idle']])
    const store = useTurnEditsStore()
    const spans = useToolSpansStore()
    await status([['a', 'working']])
    spans.upsert(edit('a', '/r/one.ts'))
    vi.advanceTimersByTime(1000)
    await status([['a', 'idle']])
    vi.advanceTimersByTime(60_000)
    await status([['a', 'working']])
    vi.advanceTimersByTime(1000)
    spans.upsert(edit('a', '/r/two.ts'))
    await nextTick()
    expect([...(store.last.get('a')?.paths ?? [])]).toEqual(['/r/one.ts'])
    vi.advanceTimersByTime(1000)
    await status([['a', 'idle']])
    expect([...(store.last.get('a')?.paths ?? [])].sort()).toEqual(['/r/one.ts', '/r/two.ts'])
    vi.advanceTimersByTime(1000)
    expect(sessionStorage.getItem('ccm.turnEdits')).toContain('/r/two.ts')
  })

  it('drops a closed terminal', async () => {
    await status([['a', 'idle']])
    const store = useTurnEditsStore()
    await status([['a', 'working']])
    useToolSpansStore().upsert(edit('a', '/r/a.ts'))
    vi.advanceTimersByTime(1000)
    await status([['a', 'idle']])
    expect(store.countOf('a')).toBe(1)
    await status([['a', 'stale']])
    expect(store.countOf('a')).toBe(0)
    expect(store.last.size).toBe(0)
  })

  it('drops a removed terminal', async () => {
    await status([['a', 'idle'], ['b', 'idle']])
    const store = useTurnEditsStore()
    await status([['a', 'working'], ['b', 'idle']])
    useToolSpansStore().upsert(edit('a', '/r/a.ts'))
    vi.advanceTimersByTime(1000)
    await status([['a', 'idle'], ['b', 'idle']])
    expect(store.countOf('a')).toBe(1)
    useTerminalsStore().remove('a')
    await nextTick()
    await nextTick()
    expect(store.last.size).toBe(0)
  })
})
