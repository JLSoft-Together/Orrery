import { afterEach, describe, expect, it } from 'vitest'
import { mount, type VueWrapper } from '@vue/test-utils'
import AppSelect from './AppSelect.vue'

const options = [
  { value: 'a', label: 'Alpha' },
  { value: 'b', label: 'Beta' },
]
let w: VueWrapper | null = null

function make(props: Record<string, unknown> = {}): VueWrapper {
  w = mount(AppSelect, {
    props: { options, label: 'Pick', modelValue: 'a', 'onUpdate:modelValue': (v: unknown) => w!.setProps({ modelValue: v }), ...props },
    attrs: { 'data-x': '1' },
    attachTo: document.body,
  })
  return w
}

afterEach(() => {
  w?.unmount()
  w = null
})

describe('AppSelect', () => {
  it('puts attrs and aria-label on the trigger and picks an option by click', async () => {
    const c = make()
    const trigger = c.find('[role=combobox]')
    expect(trigger.attributes('data-x')).toBe('1')
    expect(trigger.attributes('aria-label')).toBe('Pick')
    expect(trigger.text()).toBe('Alpha')
    await trigger.trigger('click')
    const items = document.body.querySelectorAll<HTMLElement>('[role=option]')
    expect(items).toHaveLength(2)
    items[1]!.click()
    await c.vm.$nextTick()
    expect((c.props() as { modelValue: string }).modelValue).toBe('b')
    expect(trigger.text()).toBe('Beta')
  })

  it('disabled: trigger is disabled, never opens, keys do nothing', async () => {
    const c = make({ disabled: true })
    const trigger = c.find('[role=combobox]')
    expect(trigger.attributes('disabled')).toBeDefined()
    await trigger.trigger('click')
    await trigger.trigger('keydown', { key: 'ArrowDown' })
    expect(document.body.querySelector('[role=listbox]')).toBeNull()
  })

  it('closes an open list when it becomes disabled', async () => {
    const c = make()
    await c.find('[role=combobox]').trigger('click')
    expect(document.body.querySelector('[role=listbox]')).not.toBeNull()
    await c.setProps({ disabled: true })
    await new Promise((r) => setTimeout(r, 300))
    expect(document.body.querySelector('[role=listbox]')).toBeNull()
  })

  it('keeps an explicit title attr over the default label title', () => {
    const c = make({ title: 'Folder' } as never)
    expect(c.find('[role=combobox]').attributes('title')).toBe('Folder')
  })
})
