import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import { i18n } from '../../../../i18n'
import RailCollapseButton from './RailCollapseButton.vue'

describe('RailCollapseButton', () => {
  it('names the action, reports expanded state and emits toggle', async () => {
    const w = mount(RailCollapseButton, { props: { collapsed: false }, global: { plugins: [i18n] } })
    const btn = w.get('[data-rail-collapse]')
    expect(btn.attributes('aria-label')).toMatch(/Collapse sidebar/)
    expect(btn.attributes('aria-expanded')).toBe('true')
    await btn.trigger('click')
    expect(w.emitted('toggle')).toHaveLength(1)
    await w.setProps({ collapsed: true })
    expect(btn.attributes('aria-label')).toMatch(/Expand sidebar/)
    expect(btn.attributes('aria-expanded')).toBe('false')
  })
})
