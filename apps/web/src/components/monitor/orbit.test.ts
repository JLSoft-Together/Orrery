import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { mount, type VueWrapper } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { i18n } from '../../i18n'
import { monitorGroups } from '../../lib/monitorRows'
import { useAgentsStore } from '../../stores/agents'
import { useTerminalsStore } from '../../stores/terminals'
import { useUiStore } from '../../stores/ui'
import OrbitQueue from './OrbitQueue.vue'

let w: VueWrapper | null = null

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

describe('OrbitQueue', () => {
  it('lists needs first, with icon-only agents and an orbit mini per row', () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const terminals = useTerminalsStore()
    terminals.replaceAll([
      { id: 't3', title: 'Notes', status: 'idle', cwd: '/w/app' },
      { id: 't2', title: 'Refactor', status: 'working', cwd: '/w/api', provider: 'codex' },
      { id: 't1', title: 'Fix', status: 'waiting', cwd: '/w/app', waitingFor: 'permission' },
    ])
    useAgentsStore().replaceAll([
      { id: 'a2', terminalId: 't2', role: 'main', status: 'working', currentTool: 'Edit' },
      { id: 'a3', terminalId: 't2', parentId: 'a2', role: 'subagent', status: 'working' },
    ])
    const groups = monitorGroups(terminals.list, Date.now())
    w = mount(OrbitQueue, { props: { groups, projects: [], freezeBind: {} }, global: { plugins: [i18n, pinia] }, attachTo: document.body })
    const lanes = w.findAll('[data-orbit-lane]').map((l) => l.attributes('data-orbit-lane'))
    expect(lanes).toEqual(['needs', 'running', 'idle'])
    expect(w.findAll('[data-orbit-mini]')).toHaveLength(3)
    const run = w.find('[data-orbit-row=t2]')
    expect(run.find('[data-orbit-planet]').exists()).toBe(true)
    expect(run.find('[role=img]').attributes('aria-label')).toBe('Codex')
    expect(run.text()).not.toContain('Codex')
    expect(w.find('[data-orbit-row=t3] [data-orbit-planet]').exists()).toBe(false)
  })

  it('focuses a terminal on click', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const terminals = useTerminalsStore()
    terminals.replaceAll([{ id: 't1', title: 'Fix', status: 'idle', cwd: '/w/app' }])
    w = mount(OrbitQueue, { props: { groups: [{ key: 'idle', ids: ['t1'] }], projects: [], freezeBind: {} }, global: { plugins: [i18n, pinia] }, attachTo: document.body })
    await w.find('[data-orbit-row=t1]').trigger('click')
    expect(useUiStore().focusedTerminalId).toBe('t1')
  })
})
