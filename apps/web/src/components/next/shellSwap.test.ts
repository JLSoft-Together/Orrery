import { describe, expect, it } from 'vitest'
import { defineComponent, h, nextTick, onMounted, onBeforeUnmount, ref, Teleport, watch } from 'vue'
import { mount } from '@vue/test-utils'

describe('shell swap keeps views mounted', () => {
  it('mounts the child once across classic to next and back', async () => {
    const counts = { mounted: 0, unmounted: 0 }
    const Child = defineComponent({
      setup() {
        onMounted(() => counts.mounted++)
        onBeforeUnmount(() => counts.unmounted++)
        return () => h('div', { 'data-child': '' })
      },
    })
    const Shell = defineComponent({
      setup: () => () => h('div', { class: 'shell' }, [h('div', { id: 'ccm-next-canvas' })]),
    })
    const isNext = ref(false)
    const Root = defineComponent({
      setup() {
        const hostNext = ref(isNext.value)
        watch(isNext, (v) => (hostNext.value = v), { flush: 'post' })
        return () =>
        h('div', [
          isNext.value ? h(Shell) : h('div', { id: 'ccm-classic-host', class: 'contents' }),
          h(Teleport, { to: hostNext.value ? '#ccm-next-canvas' : '#ccm-classic-host', defer: true }, [h(Child)]),
        ])
      },
    })
    const w = mount(Root, { attachTo: document.body })
    await nextTick()
    expect(counts.mounted).toBe(1)
    const el = document.querySelector('[data-child]')
    expect(document.querySelector('#ccm-classic-host [data-child]')).toBe(el)
    isNext.value = true
    await nextTick()
    await nextTick()
    expect(document.querySelector('#ccm-next-canvas [data-child]')).toBe(el)
    isNext.value = false
    await nextTick()
    await nextTick()
    expect(document.querySelector('#ccm-classic-host [data-child]')).toBe(el)
    expect(counts.mounted).toBe(1)
    expect(counts.unmounted).toBe(0)
    w.unmount()
  })
})
