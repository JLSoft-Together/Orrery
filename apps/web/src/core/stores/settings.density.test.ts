import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { nextTick } from 'vue'
import { densityOf, DENSITIES } from '../lib/density'
import { useSettingsStore } from './settings'

describe('density setting', () => {
  beforeEach(() => {
    localStorage.clear()
    document.documentElement.removeAttribute('data-density')
    setActivePinia(createPinia())
  })

  it('defaults to regular and sets data-density on the root', () => {
    const s = useSettingsStore()
    expect(s.density).toBe('regular')
    expect(document.documentElement.getAttribute('data-density')).toBe('regular')
  })

  it('reads the saved level on load', () => {
    localStorage.setItem('ccm.density', 'compact')
    const s = useSettingsStore()
    expect(s.density).toBe('compact')
    expect(document.documentElement.getAttribute('data-density')).toBe('compact')
  })

  it('falls back to regular for an unknown saved value', () => {
    localStorage.setItem('ccm.density', 'huge')
    expect(useSettingsStore().density).toBe('regular')
    expect(densityOf(null)).toBe('regular')
  })

  it('applies the attribute at once and saves ccm.density on change', async () => {
    const s = useSettingsStore()
    for (const d of DENSITIES) {
      s.density = d
      expect(document.documentElement.getAttribute('data-density')).toBe(d)
      await nextTick()
      expect(localStorage.getItem('ccm.density')).toBe(d)
    }
  })
})
