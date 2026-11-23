import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { defineComponent, h } from 'vue'
import { mount as rawMount, type VueWrapper } from '@vue/test-utils'
import { i18n } from '../../../i18n'
import { useToastsStore } from '../../../core/stores/toasts'
import { useCiStore } from '../stores/ci'
import { useCiEvents } from './useCiEvents'

const openExternal = vi.fn()
let listener: ((e: unknown) => void) | null = null

vi.mock('../../../core/lib/preview', async (orig) => ({
  ...(await orig<Record<string, unknown>>()),
  hasGhPr: true,
  ghCi: async () => ({ ok: true, watches: [{ root: 'C:\\w\\watched', branch: 'feat/w', sha: 'a', pr: null, since: 5 }] }),
  openExternal: (...a: unknown[]) => openExternal(...a),
  onGhEvent: (fn: (e: unknown) => void) => {
    listener = fn
    return () => {
      listener = null
    }
  },
}))

const Host = defineComponent({
  setup() {
    useCiEvents()
    return () => h('div')
  },
})

const wrappers: VueWrapper[] = []
const mount = (...a: Parameters<typeof rawMount>): VueWrapper => {
  const w = rawMount(...a) as VueWrapper
  wrappers.push(w)
  return w
}

const ev = (over: Record<string, unknown> = {}) => ({ type: 'ci.done', root: 'C:\\w\\p', branch: 'feat/x', sha: 'a', pr: 3, state: 'success', failed: [], url: 'https://github.com/o/r/pull/3', ...over })

describe('useCiEvents', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    openExternal.mockReset()
    i18n.global.locale.value = 'en'
    listener = null
  })

  afterEach(() => {
    for (const w of wrappers.splice(0)) w.unmount()
  })

  it('pushes a toast with an Open button when CI passes and records the state for the rail chip', () => {
    mount(Host, { global: { plugins: [i18n] } })
    listener?.(ev())
    const toast = useToastsStore().list.find((t) => t.id === 'ci-done-C:\\w\\p-feat/x')
    expect(toast?.title).toBe('CI passed')
    expect(toast?.text).toBe('feat/x')
    expect(toast?.attrs).toMatchObject({ 'data-ci-toast': 'success' })
    toast?.actions?.[0]?.run()
    expect(openExternal).toHaveBeenCalledWith('https://github.com/o/r/pull/3')
    expect(useCiStore().chip('C:\\w\\p')).toBe('success')
  })

  it('names the failed jobs and keeps the chip red', () => {
    mount(Host, { global: { plugins: [i18n] } })
    listener?.(ev({ state: 'failure', failed: ['lint', 'test'] }))
    const toast = useToastsStore().list[0]
    expect(toast?.title).toBe('CI failed')
    expect(toast?.text).toBe('feat/x: lint, test')
    expect(useCiStore().chip('C:\\w\\p')).toBe('failure')
  })

  it('never links outside github.com', () => {
    mount(Host, { global: { plugins: [i18n] } })
    listener?.(ev({ url: 'https://evil.example/x' }))
    expect(useToastsStore().list[0]?.actions).toBeUndefined()
  })

  it('a 60 minute timeout says so and clears the chip', () => {
    mount(Host, { global: { plugins: [i18n] } })
    listener?.(ev({ state: 'failure' }))
    listener?.(ev({ state: 'pending', timedOut: true }))
    expect(useToastsStore().list[0]?.title).toBe('CI still running after 60 minutes')
    expect(useCiStore().chip('C:\\w\\p')).toBeNull()
  })

  it('shows watched branches as pending on start', async () => {
    mount(Host, { global: { plugins: [i18n] } })
    await new Promise((r) => setTimeout(r, 0))
    expect(useCiStore().chip('C:\\w\\watched')).toBe('pending')
  })

  it('stops listening when the host unmounts', () => {
    const w = mount(Host, { global: { plugins: [i18n] } })
    w.unmount()
    expect(listener).toBeNull()
  })
})
