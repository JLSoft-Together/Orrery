import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { nextTick } from 'vue'
import { useWorkspaceStore } from './workspace'

const KEY = 'ccm.workspace.panes'

describe('workspace persist debounce', () => {
  beforeEach(() => {
    localStorage.clear()
    sessionStorage.clear()
    setActivePinia(createPinia())
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
    vi.restoreAllMocks()
  })

  it('merges a burst of patches into one write', async () => {
    const set = vi.spyOn(Storage.prototype, 'setItem')
    const ws = useWorkspaceStore()
    const term = ws.openBrowser('http://localhost:5173/')
    for (let i = 0; i < 20; i++) {
      ws.update(term.key, { url: `http://localhost:${5000 + i}/` })
      await nextTick()
    }
    expect(set.mock.calls.filter(([k]) => k === KEY)).toHaveLength(0)
    vi.advanceTimersByTime(500)
    expect(set.mock.calls.filter(([k]) => k === KEY)).toHaveLength(1)
    expect(JSON.parse(sessionStorage.getItem(KEY)!).panes[0].url).toBe('http://localhost:5019/')
  })

  it('writes nothing when nothing changed', async () => {
    const ws = useWorkspaceStore()
    ws.openBrowser('http://localhost:5173/')
    await nextTick()
    vi.advanceTimersByTime(500)
    const set = vi.spyOn(Storage.prototype, 'setItem')
    vi.advanceTimersByTime(2000)
    ws.persistNow()
    expect(set).not.toHaveBeenCalled()
  })

  it('flushes at once on pagehide and on beforeunload', async () => {
    const ws = useWorkspaceStore()
    ws.openBrowser('http://localhost:5173/')
    await nextTick()
    expect(sessionStorage.getItem(KEY)).toBeNull()
    window.dispatchEvent(new Event('pagehide'))
    expect(JSON.parse(sessionStorage.getItem(KEY)!).panes).toHaveLength(1)
    expect(localStorage.getItem('ccm.workspace.last')).not.toBeNull()

    sessionStorage.clear()
    ws.openFile('C:\\repo\\a.ts', 1)
    await nextTick()
    window.dispatchEvent(new Event('beforeunload'))
    expect(JSON.parse(sessionStorage.getItem(KEY)!).panes).toHaveLength(2)
  })

  it('a pane closed inside the debounce is not written', async () => {
    const ws = useWorkspaceStore()
    const a = ws.openBrowser('http://localhost:5173/')
    ws.openFile('C:\\repo\\a.ts', 1)
    await nextTick()
    ws.remove(a.key)
    await nextTick()
    vi.advanceTimersByTime(500)
    expect(JSON.parse(sessionStorage.getItem(KEY)!).panes.map((p: { key: string }) => p.key)).not.toContain(a.key)
  })
})
