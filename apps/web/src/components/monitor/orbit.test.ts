import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { nextTick } from 'vue'
import { mount, type VueWrapper } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { i18n } from '../../i18n'
import { monitorGroups } from '../../lib/monitorRows'
import { useAgentsStore } from '../../stores/agents'
import { useSettingsStore } from '../../stores/settings'
import { useTerminalsStore } from '../../stores/terminals'
import { useToolSpansStore } from '../../stores/toolSpans'
import { useUiStore } from '../../stores/ui'
import OrbitView from './OrbitView.vue'

let w: VueWrapper | null = null

beforeEach(() => {
  localStorage.clear()
  i18n.global.locale.value = 'en'
})
afterEach(() => {
  w?.unmount()
  w = null
  document.body.innerHTML = ''
})

function seed() {
  const pinia = createPinia()
  setActivePinia(pinia)
  const terminals = useTerminalsStore()
  terminals.replaceAll([
    { id: 't3', title: 'Notes', status: 'idle', cwd: '/w/app' },
    { id: 't2', title: 'Refactor', status: 'working', cwd: '/w/api', provider: 'codex' },
    { id: 't1', title: 'Fix', status: 'waiting', cwd: '/w/app', waitingFor: 'permission' },
  ])
  useAgentsStore().replaceAll([
    { id: 'a1', terminalId: 't1', role: 'main', status: 'waiting', currentTool: 'Bash', currentToolSummary: 'cargo build --release' },
    { id: 'a2', terminalId: 't2', role: 'main', status: 'working', currentTool: 'Edit' },
    { id: 'a3', terminalId: 't2', parentId: 'a2', role: 'subagent', status: 'working', type: 'Explore', currentTool: 'Grep' },
  ])
  const groups = monitorGroups(terminals.list, Date.now())
  w = mount(OrbitView, { props: { groups }, global: { plugins: [i18n, pinia] }, attachTo: document.body })
  return pinia
}

const node = (id: string) => w!.find(`[data-orbit-node=${id}]`)

describe('OrbitView', () => {
  it('draws one node per session with agent icon only, ring by state, and sub-agent dots', () => {
    seed()
    expect(w!.findAll('[data-orbit-node]').map((n) => n.attributes('data-orbit-node'))).toEqual(['t1', 't2', 't3'])
    expect(node('t1').attributes('data-ring')).toBe('0')
    expect(node('t2').attributes('data-ring')).toBe('1')
    expect(node('t3').attributes('data-ring')).toBe('2')
    expect(node('t2').find('[role=img]').exists()).toBe(false)
    expect(node('t2').attributes('aria-label')).toContain('Codex')
    expect(w!.findAll('[data-orbit-sub]')).toHaveLength(1)
    expect(w!.findAll('[data-orbit-sector]').map((s) => s.text()).sort()).toEqual(['api', 'app'])
  })

  it('selects the neediest session first and shows its detail panel', () => {
    seed()
    expect(node('t1').attributes('aria-pressed')).toBe('true')
    const detail = w!.find('[data-orbit-detail]')
    expect(detail.find('[data-detail-title]').text()).toBe('Fix')
    expect(detail.find('[data-detail-needs]').text()).toContain('cargo build --release')
  })

  it('selects on click and on arrow keys', async () => {
    seed()
    await node('t2').trigger('click')
    expect(node('t2').attributes('aria-pressed')).toBe('true')
    expect(w!.find('[data-detail-title]').text()).toBe('Refactor')
    expect(w!.find('[data-detail-sub]').text()).toContain('Explore')
    await node('t2').trigger('keydown', { key: 'ArrowLeft' })
    expect(w!.findAll('[aria-pressed=true]')).toHaveLength(1)
  })

  it('opens the focus view on Enter or double click and leaves with Esc', async () => {
    seed()
    expect(w!.find('[data-orbit-focus]').exists()).toBe(false)
    await node('t2').trigger('keydown', { key: 'Enter' })
    expect(w!.find('[data-orbit-focus]').exists()).toBe(true)
    expect(w!.find('[data-focus-title]').text()).toBe('Refactor')
    await w!.find('[data-orbit-focus]').trigger('keydown', { key: 'Escape' })
    expect(w!.find('[data-orbit-focus]').exists()).toBe(false)

    await node('t3').trigger('dblclick')
    expect(w!.find('[data-focus-title]').text()).toBe('Notes')
    await w!.find('[data-focus-back]').trigger('click')
    expect(w!.find('[data-orbit-focus]').exists()).toBe(false)
  })

  it('jumps between systems from the rail, needs first', async () => {
    seed()
    await node('t2').trigger('dblclick')
    const rail = w!.findAll('[data-focus-rail]').filter((n) => n.element.tagName === 'BUTTON').map((n) => n.attributes('data-focus-rail'))
    expect(rail).toEqual(['t1', 't2', 't3'])
    await w!.find('[data-focus-rail=t1]').trigger('click')
    expect(w!.find('[data-focus-title]').text()).toBe('Fix')
    await w!.find('[data-focus-next]').trigger('click')
    expect(w!.find('[data-focus-title]').text()).toBe('Refactor')
  })

  it('shows sub-agents on the planet orbits and tool calls in the feed, newest first', async () => {
    const pinia = seed()
    const now = Date.now()
    const spans = useToolSpansStore(pinia)
    spans.upsert({ id: 's1', terminalId: 't2', agentId: 'a2', tool: 'Read', start: new Date(now - 90_000).toISOString(), end: new Date(now - 88_000).toISOString(), outcome: 'ok' })
    spans.upsert({ id: 's2', terminalId: 't2', agentId: 'a3', tool: 'Grep', start: new Date(now - 10_000).toISOString() })
    await node('t2').trigger('dblclick')
    await nextTick()
    expect(w!.find('[data-focus-planet=a3]').exists()).toBe(true)
    expect(w!.find('[data-focus-trail=a3]').exists()).toBe(true)
    const rows = w!.findAll('[data-focus-row]').map((r) => r.text())
    expect(rows).toHaveLength(2)
    expect(rows[0]).toContain('Grep')
    expect(rows[0]).toContain('Explore')
    expect(rows[1]).toContain('Read')
  })

  it('keeps the Map and Office switch, only when Office is enabled', async () => {
    const pinia = seed()
    expect(w!.find('[role=radiogroup]').exists()).toBe(false)
    useSettingsStore(pinia).officeEnabled = true
    await nextTick()
    const radios = w!.findAll('[role=radio]')
    expect(radios).toHaveLength(2)
    await radios[1]!.trigger('click')
    expect(useUiStore(pinia).mapView).toBe('office')
  })

  it('goes to the sheet layout on a narrow window', async () => {
    const pinia = seed()
    useUiStore(pinia).narrow = true
    await nextTick()
    expect(w!.find('[data-orbit-panel]').classes()).toContain('absolute')
    await w!.find('[data-detail-close]').trigger('click')
    expect(w!.find('[data-orbit-panel]').exists()).toBe(false)
  })
})
