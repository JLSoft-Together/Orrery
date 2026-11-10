import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { defineComponent, h, nextTick } from 'vue'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { useMonitorSessions } from '../../composables/useMonitorSessions'
import { i18n } from '../../i18n'
import { now } from '../../lib/format'
import { useAgentsStore } from '../../stores/agents'
import { useTerminalsStore } from '../../stores/terminals'
import { useUiStore } from '../../stores/ui'
import CommandPanel from './CommandPanel.vue'
import ModeBar from './ModeBar.vue'
import SessionBoard from './SessionBoard.vue'

let w: VueWrapper | null = null
const iso = (s: number) => new Date(Date.now() - s * 1000).toISOString()

beforeEach(() => {
  localStorage.clear()
  setActivePinia(createPinia())
  i18n.global.locale.value = 'en'
})
afterEach(() => {
  w?.unmount()
  w = null
  document.body.innerHTML = ''
})

function seed(): void {
  useTerminalsStore().replaceAll([
    { id: 't1', title: 'Fix login', status: 'waiting', cwd: '/w/app', waitingFor: 'permission', statusSince: iso(90) },
    { id: 't2', title: 'Refactor', status: 'working', cwd: '/w/api', provider: 'codex' },
    { id: 't3', title: 'Notes', status: 'idle', cwd: '/w/app' },
  ])
  useAgentsStore().replaceAll([
    { id: 'a1', terminalId: 't1', role: 'main', status: 'waiting', currentTool: 'Bash', currentToolSummary: 'npm test -- --run', currentToolStartedAt: iso(30) },
    { id: 'a2', terminalId: 't2', role: 'main', status: 'working', currentTool: 'Edit' },
    { id: 'a3', terminalId: 't2', parentId: 'a2', role: 'subagent', status: 'working' },
  ])
}

describe('ModeBar', () => {
  it('renders the two tabs and switches mode', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    w = mount(ModeBar, { props: { projects: [], groups: [{ key: 'needs', ids: ['a', 'b'] }] }, global: { plugins: [i18n, pinia] } })
    const tabs = w.findAll('[role=tab]')
    expect(tabs.map((t) => t.text())).toEqual(['Board', 'Sky'])
    expect(tabs[0]!.attributes('aria-selected')).toBe('true')
    await tabs[1]!.trigger('click')
    expect(useUiStore().monitorMode).toBe('sky')
    expect(w.find('[data-chip=needs]').text()).toContain('2')
  })
})

describe('SessionBoard', () => {
  it('shows four columns with icon-only agents and an open-pane button for needs', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    seed()
    w = mount(SessionBoard, {
      props: {
        groups: [
          { key: 'needs', ids: ['t1'] },
          { key: 'running', ids: ['t2'] },
          { key: 'idle', ids: ['t3'] },
        ],
        freezeBind: {},
      },
      global: { plugins: [i18n, pinia] },
    })
    await flushPromises()
    expect(w.findAll('[data-board-column]').map((c) => c.attributes('data-board-column'))).toEqual(['needs', 'running', 'idle', 'ended'])
    const card = w.find('[data-board-card=t2]')
    expect(card.attributes('aria-label')).toContain('Codex')
    expect(card.text()).not.toContain('Codex')
    expect(card.find('[data-board-subs]').text()).toContain('1')
    expect(w.find('[data-board-card=t1] [data-open-pane]').exists()).toBe(false)
    expect(w.find('[data-board-column=ended]').text()).toContain('No sessions')
  })
})

describe('CommandPanel', () => {
  it('keeps the 32px bar and toggles the table with aria-expanded', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    seed()
    w = mount(CommandPanel, { global: { plugins: [i18n, pinia] } })
    const toggle = w.find('[data-cmd-toggle]')
    expect(toggle.attributes('aria-expanded')).toBe('false')
    expect(w.find('[data-cmd-count]').text()).toBe('2 running')
    expect(w.find('[data-cmd-longest]').text()).toContain('npm test -- --run')
    expect(w.find('[data-cmd-body]').exists()).toBe(false)
    await toggle.trigger('click')
    expect(toggle.attributes('aria-expanded')).toBe('true')
    expect(localStorage.getItem('ccm.cmdPanelOpen')).toBe('1')
    expect(w.findAll('[data-cmd-row]')).toHaveLength(2)
  })

  it('tints a waiting row and falls back to the tool name without a summary or start', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    seed()
    useUiStore().cmdPanelOpen = true
    w = mount(CommandPanel, { global: { plugins: [i18n, pinia] } })
    const waiting = w.find('[data-cmd-row=a1]')
    expect(waiting.attributes('data-cmd-waiting')).toBeDefined()
    expect(waiting.classes()).toContain('bg-warning-soft')
    const plain = w.find('[data-cmd-row=a2]')
    expect(plain.find('[data-cmd-text]').text()).toBe('Edit')
    expect(plain.find('[data-cmd-time]').text()).toBe('unknown')
    expect(plain.attributes('data-cmd-waiting')).toBeUndefined()
  })
})

describe('BoardCard', () => {
  it('opens on Enter / Space from the card itself, not from its button', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    seed()
    const ui = useUiStore()
    const { default: BoardCard } = await import('./BoardCard.vue')
    w = mount(BoardCard, { props: { terminal: useTerminalsStore().byId['t2']! }, global: { plugins: [i18n, pinia] } })
    await w.trigger('keydown', { key: 'Enter' })
    expect(ui.focusedTerminalId).toBe('t2')
    ui.focusedTerminalId = null
    await w.trigger('keydown', { key: ' ' })
    expect(ui.focusedTerminalId).toBe('t2')
    ui.focusedTerminalId = null
    const inner = document.createElement('button')
    w.element.appendChild(inner)
    inner.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }))
    expect(ui.focusedTerminalId).toBeNull()
  })

  it('shows only the tool when the start time cannot be read', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    useTerminalsStore().replaceAll([{ id: 't9', title: 'x', status: 'working', cwd: '/w/x' }])
    useAgentsStore().replaceAll([{ id: 'a9', terminalId: 't9', role: 'main', status: 'working', currentTool: 'Bash', currentToolStartedAt: 'not a date' }])
    const { default: BoardCard } = await import('./BoardCard.vue')
    w = mount(BoardCard, { props: { terminal: useTerminalsStore().byId['t9']! }, global: { plugins: [i18n, pinia] } })
    expect(w.find('[data-board-line]').text()).toBe('Bash')
  })
})

describe('useMonitorSessions', () => {
  function harness() {
    const pinia = createPinia()
    setActivePinia(pinia)
    let api!: ReturnType<typeof useMonitorSessions>
    const C = defineComponent({
      setup() {
        api = useMonitorSessions()
        return () => h('div')
      },
    })
    w = mount(C, { global: { plugins: [pinia] } })
    return api
  }
  const at = (m: number) => new Date(Date.UTC(2026, 10, 11, 10, m)).toISOString()

  it('releases the frozen order when the mode changes', async () => {
    const api = harness()
    const terminals = useTerminalsStore()
    terminals.replaceAll([
      { id: 'A', title: 'A', status: 'idle', lastActivityAt: at(0) },
      { id: 'B', title: 'B', status: 'idle', lastActivityAt: at(5) },
    ])
    expect(api.sessionGroups.value[0]!.ids).toEqual(['B', 'A'])
    api.freezeBind.onFocusin()
    terminals.replaceAll([
      { id: 'A', title: 'A', status: 'idle', lastActivityAt: at(30) },
      { id: 'B', title: 'B', status: 'idle', lastActivityAt: at(5) },
      { id: 'C', title: 'C', status: 'idle', lastActivityAt: at(40) },
    ])
    expect(api.sessionGroups.value[0]!.ids).toEqual(['B', 'A', 'C'])
    useUiStore().monitorMode = 'sky'
    await nextTick()
    expect(api.sessionGroups.value[0]!.ids).toEqual(['C', 'A', 'B'])
  })

  it('releases the frozen order on unmount', () => {
    const api = harness()
    const terminals = useTerminalsStore()
    terminals.replaceAll([{ id: 'A', title: 'A', status: 'idle', lastActivityAt: at(0) }])
    api.freezeBind.onFocusin()
    w!.unmount()
    w = null
    terminals.replaceAll([
      { id: 'A', title: 'A', status: 'idle', lastActivityAt: at(0) },
      { id: 'B', title: 'B', status: 'idle', lastActivityAt: at(9) },
    ])
    expect(api.sessionGroups.value[0]!.ids).toEqual(['B', 'A'])
  })

  it('drops yesterday sessions from Done today when the day changes', async () => {
    const api = harness()
    const day1 = new Date(2026, 10, 11, 12)
    now.value = day1.getTime()
    useTerminalsStore().replaceAll([{ id: 'E', title: 'E', status: 'stale', endedAt: new Date(2026, 10, 11, 9).toISOString() }])
    expect(api.sessionGroups.value.map((g) => g.key)).toEqual(['ended'])
    now.value = new Date(2026, 10, 12, 0, 1).getTime()
    await nextTick()
    expect(api.sessionGroups.value).toEqual([])
  })

  it('filters by project only on the board', () => {
    const api = harness()
    const ui = useUiStore()
    useTerminalsStore().replaceAll([
      { id: 'A', title: 'A', status: 'idle', cwd: '/w/app' },
      { id: 'B', title: 'B', status: 'idle', cwd: '/w/api' },
    ])
    ui.monitorProject = api.groups.value[0]!.key
    expect(api.sessionGroups.value[0]!.ids).toHaveLength(1)
    ui.monitorMode = 'sky'
    expect(api.sessionGroups.value[0]!.ids).toHaveLength(2)
  })
})
