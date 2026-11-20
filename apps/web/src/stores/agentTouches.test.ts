import { beforeEach, describe, expect, it } from 'vitest'
import { nextTick } from 'vue'
import { createPinia, setActivePinia } from 'pinia'
import type { TerminalSession, ToolSpan } from '@ccm/shared'
import { useAgentTouchesStore } from './agentTouches'
import { useSettingsStore } from './settings'
import { useTerminalsStore } from './terminals'
import { useToolSpansStore } from './toolSpans'

const term = (id: string, status: TerminalSession['status']): TerminalSession => ({ id, title: id, status, startedAt: '2026-11-20T09:00:00.000Z' }) as TerminalSession
let seq = 0
const edit = (terminalId: string, path: string): ToolSpan => ({ id: `s${++seq}`, terminalId, agentId: `${terminalId}:main`, tool: 'Edit', start: new Date().toISOString(), path, access: 'edit' })

async function setup(): Promise<ReturnType<typeof useAgentTouchesStore>> {
  const terminals = useTerminalsStore()
  terminals.replaceAll([term('a', 'idle'), term('b', 'idle')])
  const store = useAgentTouchesStore()
  await nextTick()
  terminals.replaceAll([term('a', 'working'), term('b', 'working')])
  await nextTick()
  const spans = useToolSpansStore()
  spans.upsert(edit('a', '/p/a.ts'))
  spans.upsert(edit('b', '/p/a.ts'))
  return store
}

describe('agentTouches clashes', () => {
  beforeEach(() => {
    localStorage.clear()
    setActivePinia(createPinia())
  })

  it('stays empty while the switch is off (default)', async () => {
    const store = await setup()
    expect(useSettingsStore().clashWarn).toBe(false)
    expect(store.index.files.get('/p/a.ts')?.state).toBe('conflict')
    expect(store.clashes).toEqual([])
    expect(store.takeNew()).toEqual([])
  })

  it('reports once per turn, and ignore hides the file until undone', async () => {
    useSettingsStore().clashWarn = true
    const store = await setup()
    expect(store.clashes.map((c) => c.fileKey)).toEqual(['/p/a.ts'])
    expect(store.clashesOf.get('a')).toHaveLength(1)
    expect(store.takeNew()).toHaveLength(1)
    expect(store.takeNew()).toHaveLength(0)
    store.ignoreFile('/p/a.ts')
    expect(store.clashes).toEqual([])
    const saved = JSON.parse(localStorage.getItem('ccm.clash.ignored') ?? '[]') as { key: string; ids: string[] }[]
    expect(saved.map((x) => ({ key: x.key, ids: [...x.ids].sort() }))).toEqual([{ key: '/p/a.ts', ids: ['a', 'b'] }])
    store.unignoreFile('/p/a.ts')
    expect(store.clashes).toHaveLength(1)
    expect(store.takeNew()).toHaveLength(0)
  })

  it('a new turn of one side reports again', async () => {
    useSettingsStore().clashWarn = true
    const store = await setup()
    expect(store.takeNew()).toHaveLength(1)
    const terminals = useTerminalsStore()
    terminals.replaceAll([term('a', 'idle'), term('b', 'working')])
    await nextTick()
    await new Promise((r) => setTimeout(r, 5))
    terminals.replaceAll([term('a', 'working'), term('b', 'working')])
    await nextTick()
    useToolSpansStore().upsert(edit('a', '/p/a.ts'))
    expect(store.takeNew()).toHaveLength(1)
  })
})
