import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { nextTick } from 'vue'

vi.mock('../lib/desktop', async (original) => ({ ...(await original<typeof import('../lib/desktop')>()), hasTerminals: true }))

const { useUiStore } = await import('./ui')

describe('start view', () => {
  beforeEach(() => {
    localStorage.clear()
    sessionStorage.clear()
    setActivePinia(createPinia())
  })

  it('opens on the Workspace, even when the last run ended on Monitor', () => {
    localStorage.setItem('ccm.view', 'monitor')
    expect(useUiStore().view).toBe('workspace')
  })

  it('keeps the current view across a reload in the same window', () => {
    sessionStorage.setItem('ccm.view', 'usage')
    expect(useUiStore().view).toBe('usage')
  })

  it('remembers a switch only for this window', async () => {
    useUiStore().setView('history')
    await nextTick()
    expect(sessionStorage.getItem('ccm.view')).toBe('history')
    expect(localStorage.getItem('ccm.view')).toBeNull()
  })
})

describe('monitor mode and command panel', () => {
  beforeEach(() => {
    localStorage.clear()
    setActivePinia(createPinia())
  })

  it('defaults to the board with the panel closed', () => {
    const ui = useUiStore()
    expect(ui.monitorMode).toBe('board')
    expect(ui.cmdPanelOpen).toBe(false)
  })

  it('persists the mode and the panel state', async () => {
    const ui = useUiStore()
    ui.monitorMode = 'sky'
    ui.cmdPanelOpen = true
    await nextTick()
    expect(localStorage.getItem('ccm.monitorMode')).toBe('sky')
    expect(localStorage.getItem('ccm.cmdPanelOpen')).toBe('1')
    setActivePinia(createPinia())
    const again = useUiStore()
    expect(again.monitorMode).toBe('sky')
    expect(again.cmdPanelOpen).toBe(true)
  })

  it('ignores an unknown stored mode', () => {
    localStorage.setItem('ccm.monitorMode', 'radar')
    expect(useUiStore().monitorMode).toBe('board')
  })
})

describe('compact view', () => {
  beforeEach(() => {
    localStorage.clear()
    history.replaceState(null, '', '/')
    setActivePinia(createPinia())
  })

  it('leaveCompact returns an in-window compact view to the full view', () => {
    const ui = useUiStore()
    ui.toggleCompact()
    expect(ui.compact).toBe(true)
    ui.leaveCompact()
    expect(ui.compact).toBe(false)
  })

  it('a window opened as compact asks the host for the main window instead', () => {
    history.replaceState(null, '', '/?mode=compact')
    const open = vi.spyOn(window, 'open').mockReturnValue(null)
    const ui = useUiStore()
    expect(ui.compact).toBe(true)
    ui.leaveCompact()
    expect(open).toHaveBeenCalledWith(`${location.origin}/`, '_blank')
    expect(ui.compact).toBe(true)
    open.mockRestore()
  })
})
