import { afterEach, describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import GlassMenu from './GlassMenu.vue'

const items = [
  { id: 'a', label: 'A' },
  { id: 'b', label: 'B' },
]

function menu() {
  return mount(GlassMenu, { props: { open: true, items, label: 'More', x: 10, y: 10 }, attachTo: document.body, global: { stubs: { Transition: false } } })
}

afterEach(() => {
  document.body.innerHTML = ''
})

describe('GlassMenu dismissal', () => {
  it('closes on a pointerdown outside, not inside', async () => {
    const w = menu()
    await nextTick()
    document.body.querySelector('[data-glass-menu]')!.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }))
    expect(w.emitted('close')).toBeUndefined()
    document.body.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }))
    expect(w.emitted('close')).toHaveLength(1)
    w.unmount()
  })

  it('closes on Escape anywhere and when the window loses focus', async () => {
    const w = menu()
    await nextTick()
    document.body.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))
    expect(w.emitted('close')).toHaveLength(1)
    window.dispatchEvent(new Event('blur'))
    expect(w.emitted('close')).toHaveLength(2)
    w.unmount()
  })

  it('stops listening once closed', async () => {
    const w = menu()
    await nextTick()
    await w.setProps({ open: false })
    document.body.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }))
    window.dispatchEvent(new Event('blur'))
    expect(w.emitted('close')).toBeUndefined()
    w.unmount()
  })
})
