import { beforeEach, describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { i18n } from '../../i18n'
import { useExtrasStore } from '../../stores/extras'
import NextQuotaBar from './NextQuotaBar.vue'

const fresh = (): string => new Date(Date.now() - 60_000).toISOString()

function make() {
  const pinia = createPinia()
  setActivePinia(pinia)
  const extras = useExtrasStore()
  const w = mount(NextQuotaBar, { global: { plugins: [pinia, i18n] } })
  return { w, extras }
}

describe('NextQuotaBar', () => {
  beforeEach(() => localStorage.clear())

  it('shows only the placeholder while no agent reports', () => {
    const { w } = make()
    expect(w.find('[data-next-footer]').exists()).toBe(true)
    expect(w.findAll('[data-quota-agent]')).toHaveLength(0)
    expect(w.find('[data-quota-empty]').exists()).toBe(true)
  })

  it('renders one cluster per agent with its percentages and reset time', async () => {
    const { w, extras } = make()
    const reset = new Date(Date.now() + 2 * 3_600_000).toISOString()
    extras.limits = { updatedAt: fresh(), fiveHour: { usedPct: 42, resetsAt: reset }, sevenDay: { usedPct: 91 } }
    extras.codexLimits = { updatedAt: fresh(), fiveHour: { usedPct: 7 } }
    await w.vm.$nextTick()
    const agents = w.findAll('[data-quota-agent]')
    expect(agents.map((a) => a.attributes('data-quota-agent'))).toEqual(['claude', 'codex'])
    expect(agents[0]!.text()).toContain('Claude Code')
    expect(agents[0]!.text()).toContain('42%')
    expect(agents[0]!.text()).toContain('91%')
    expect(agents[0]!.text()).toContain('reset')
    expect(agents[1]!.text()).toContain('Codex')
    expect(agents[1]!.findAll('[data-quota]')).toHaveLength(1)
    expect(w.find('[data-quota-empty]').exists()).toBe(false)
  })

  it('marks an agent stale and leaves a missing one out', async () => {
    const { w, extras } = make()
    extras.limits = { updatedAt: new Date(Date.now() - 40 * 60_000).toISOString(), fiveHour: { usedPct: 10 } }
    await w.vm.$nextTick()
    expect(w.findAll('[data-quota-agent]')).toHaveLength(1)
    expect(w.find('[data-quota-agent="claude"]').attributes('data-stale')).toBe('true')
    expect(w.find('[data-quota-agent="codex"]').exists()).toBe(false)
  })
})
