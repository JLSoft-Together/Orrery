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

  it('renders one cluster per agent with its percentages, reset time only in the tooltip', async () => {
    const { w, extras } = make()
    const reset = new Date(Date.now() + 2 * 3_600_000).toISOString()
    extras.limits = { updatedAt: fresh(), fiveHour: { usedPct: 42, resetsAt: reset }, sevenDay: { usedPct: 91 } }
    extras.codexLimits = { updatedAt: fresh(), fiveHour: { usedPct: 7 } }
    await w.vm.$nextTick()
    const agents = w.findAll('[data-quota-agent]')
    expect(agents.map((a) => a.attributes('data-quota-agent'))).toEqual(['claude', 'codex'])
    expect(agents[0]!.text()).toContain('42%')
    expect(agents[0]!.text()).toContain('91%')
    expect(agents[0]!.text()).not.toContain('reset')
    expect(agents[0]!.element.tagName).toBe('BUTTON')
    expect(agents[0]!.attributes('title')).toContain('reset')
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

  it('opens a popover with every window of every agent on click and closes on Escape and outside click', async () => {
    const { w, extras } = make()
    const reset = new Date(Date.now() + 2 * 3_600_000 + 30_000).toISOString()
    extras.limits = { updatedAt: fresh(), fiveHour: { usedPct: 42, resetsAt: reset }, sevenDay: { usedPct: 95 } }
    extras.codexLimits = { updatedAt: new Date(Date.now() - 40 * 60_000).toISOString(), fiveHour: { usedPct: 85 } }
    await w.vm.$nextTick()
    expect(w.find('[data-quota-popover]').exists()).toBe(false)
    await w.find('[data-quota-agent="claude"]').trigger('click')
    const pop = w.find('[data-quota-popover]')
    expect(pop.exists()).toBe(true)
    expect(w.find('[data-quota-agent="claude"]').attributes('aria-expanded')).toBe('true')
    expect(pop.findAll('[data-quota-window]')).toHaveLength(3)
    expect(pop.find('[data-quota-detail="claude"]').text()).toContain('Claude Code')
    expect(pop.find('[data-quota-window="5h"] [data-quota-reset]').text()).toMatch(/2 h/)
    expect(pop.find('[data-quota-window="7d"] [data-quota-level-word]').text()).toBe('Near the limit')
    expect(pop.find('[data-quota-window="5h"] [data-quota-level-word]').text()).toBe('OK')
    expect(pop.find('[data-quota-detail="codex"] [data-quota-level-word]').text()).toBe('Running low')
    expect(pop.find('[data-quota-detail-stale]').exists()).toBe(true)
    document.body.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))
    await w.vm.$nextTick()
    expect(w.find('[data-quota-popover]').exists()).toBe(false)
    await w.find('[data-quota-agent="codex"]').trigger('click')
    expect(w.find('[data-quota-popover]').exists()).toBe(true)
    document.body.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }))
    await w.vm.$nextTick()
    expect(w.find('[data-quota-popover]').exists()).toBe(false)
  })

  it('shows a short level word in the footer only for a critical window', async () => {
    const { w, extras } = make()
    extras.limits = { updatedAt: fresh(), fiveHour: { usedPct: 50 }, sevenDay: { usedPct: 95 } }
    await w.vm.$nextTick()
    expect(w.find('[data-quota="5h"] [data-quota-level]').exists()).toBe(false)
    expect(w.find('[data-quota="7d"] [data-quota-level]').exists()).toBe(true)
  })
})
