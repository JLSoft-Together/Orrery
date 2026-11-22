import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { nextTick } from 'vue'
import { useSettingsStore } from './settings'
import { useUiGenStore } from './uiGen'

const root = document.documentElement

describe('uiGen store', () => {
  beforeEach(() => {
    localStorage.clear()
    localStorage.setItem('ccm.theme', 'orrery-light')
    localStorage.setItem('ccm.themeMigration', '4.0.2-galaxy')
    localStorage.setItem('ccm.uiMigration', '4.1-next')
    localStorage.setItem('ccm.uiGeneration', 'classic')
    root.removeAttribute('data-theme')
    root.removeAttribute('data-ui')
    root.removeAttribute('data-next-theme')
    root.removeAttribute('data-glass')
    root.className = ''
    root.style.cssText = ''
    setActivePinia(createPinia())
  })

  it('keeps a stored classic choice after the migration and leaves the classic theme on the root', () => {
    const settings = useSettingsStore()
    const gen = useUiGenStore()
    expect(gen.generation).toBe('classic')
    expect(root.dataset.ui).toBe('classic')
    expect(root.dataset.theme).toBe(settings.theme)
    expect(root.dataset.nextTheme).toBeUndefined()
  })

  it('fresh install starts on next and records the migration', () => {
    localStorage.clear()
    const gen = useUiGenStore()
    expect(gen.generation).toBe('next')
    expect(localStorage.getItem('ccm.uiGeneration')).toBe('next')
    expect(localStorage.getItem('ccm.uiMigration')).toBe('4.1-next')
  })

  it('moves a machine on classic to next once, then respects a later classic choice', async () => {
    localStorage.clear()
    localStorage.setItem('ccm.uiGeneration', 'classic')
    const first = useUiGenStore()
    expect(first.generation).toBe('next')
    expect(localStorage.getItem('ccm.uiMigration')).toBe('4.1-next')
    first.setGeneration('classic')
    await nextTick()
    expect(localStorage.getItem('ccm.uiGeneration')).toBe('classic')
    setActivePinia(createPinia())
    expect(useUiGenStore().generation).toBe('classic')
  })

  it('switches to next and back at once, keeping the classic theme', async () => {
    const settings = useSettingsStore()
    const gen = useUiGenStore()
    gen.setGeneration('next')
    expect(root.dataset.ui).toBe('next')
    expect(root.dataset.theme).toBeUndefined()
    expect(root.dataset.nextTheme).toBe('mist')
    await nextTick()
    expect(localStorage.getItem('ccm.uiGeneration')).toBe('next')
    expect(root.style.getPropertyValue('--font-sans')).toContain('Manrope Variable')
    gen.toggleGeneration()
    expect(root.dataset.ui).toBe('classic')
    expect(root.dataset.theme).toBe(settings.theme)
    expect(root.dataset.nextTheme).toBeUndefined()
    await nextTick()
    expect(root.style.getPropertyValue('--font-sans')).not.toContain('Manrope')
  })

  it('a dark-only palette forces dark, light follows the classic mode', () => {
    const gen = useUiGenStore()
    gen.setGeneration('next')
    gen.setNextTheme('galaxy')
    expect(root.classList.contains('dark')).toBe(true)
    gen.setNextTheme('mono')
    expect(root.classList.contains('dark')).toBe(false)
    gen.setNextTheme('nope')
    expect(gen.nextTheme).toBe('mono')
  })

  it('custom theme seeds from the previous palette, sets vars inline only while active, persists debounced', async () => {
    const gen = useUiGenStore()
    gen.setGeneration('next')
    gen.setNextTheme('graphite')
    expect(root.style.getPropertyValue('--ccm-accent')).toBe('')
    gen.setNextTheme('custom')
    expect(gen.nextTheme).toBe('custom')
    expect(root.dataset.nextTheme).toBe('custom')
    expect(gen.custom.seed).toBe('graphite')
    expect(root.style.getPropertyValue('--ccm-accent')).toBe(gen.customScheme.vars['--ccm-accent'])
    gen.setCustom({ accent: '#c15f3c' })
    expect(root.style.getPropertyValue('--ccm-accent')).toBe(gen.customScheme.vars['--ccm-accent'])
    await new Promise((r) => setTimeout(r, 260))
    expect(JSON.parse(localStorage.getItem('ccm.next.custom')!).accent).toBe('#c15f3c')
    gen.reduceGlass = true
    await nextTick()
    expect(root.style.getPropertyValue('--ccm-glass')).toBe('')
    gen.setNextTheme('mono')
    expect(root.style.getPropertyValue('--ccm-accent')).toBe('')
    gen.setNextTheme('custom')
    expect(gen.custom.accent).toBe('#c15f3c')
    gen.setGeneration('classic')
    expect(root.style.getPropertyValue('--ccm-accent')).toBe('')
  })

  it('a chosen UI font wins over Manrope', async () => {
    const settings = useSettingsStore()
    const gen = useUiGenStore()
    gen.setGeneration('next')
    settings.setUiFont('inter')
    await nextTick()
    expect(root.style.getPropertyValue('--font-sans')).toContain('Inter Variable')
  })

  it('persists the next keys and sets glass / motion on the root', async () => {
    const gen = useUiGenStore()
    gen.reduceGlass = true
    gen.railGroup = 'status'
    gen.explorerPinned = true
    gen.clearTerm = true
    gen.setMotion('off')
    await nextTick()
    expect(root.dataset.glass).toBe('reduced')
    expect(root.dataset.motion).toBe('off')
    expect(localStorage.getItem('ccm.next.reduceGlass')).toBe('1')
    expect(localStorage.getItem('ccm.next.railGroup')).toBe('status')
    expect(localStorage.getItem('ccm.next.explorerPinned')).toBe('1')
    expect(localStorage.getItem('ccm.next.clearTerm')).toBe('1')
    expect(localStorage.getItem('ccm.next.motion')).toBe('off')
    gen.reduceGlass = false
    await nextTick()
    expect(root.dataset.glass).toBeUndefined()
  })

  it('an older palette id stays selected and applies', () => {
    for (const id of ['nebula', 'aurora', 'dawn', 'console']) {
      localStorage.setItem('ccm.nextTheme', id)
      setActivePinia(createPinia())
      const gen = useUiGenStore()
      expect(gen.nextTheme).toBe(id)
      expect(localStorage.getItem('ccm.nextTheme')).toBe(id)
      gen.setGeneration('next')
      expect(root.dataset.nextTheme).toBe(id)
    }
  })

  it('random mixes a palette inline, mixes again, keeps a seed and rolls at the next launch only while not kept', async () => {
    const gen = useUiGenStore()
    gen.setGeneration('next')
    gen.setNextTheme('random')
    expect(root.dataset.nextTheme).toBe('random')
    const first = gen.random!
    expect(root.style.getPropertyValue('--ccm-accent')).toBe(gen.randomTheme!.css[gen.scheme]['--ccm-accent'])
    expect(root.style.getPropertyValue('--ccm-ansi-1')).toBeTruthy()
    await nextTick()
    const saved = JSON.parse(localStorage.getItem('ccm.next.randomPalette')!)
    expect(saved.seed).toBe(first.seed)
    expect(saved.css.light['--ccm-canvas']).toBe(gen.randomTheme!.css.light['--ccm-canvas'])
    expect(saved.upcomingCss.dark['--ccm-accent']).toBeTruthy()

    setActivePinia(createPinia())
    const next = useUiGenStore()
    expect(next.random!.seed).toBe(first.upcoming)
    expect(root.style.getPropertyValue('--ccm-accent')).toBe(next.randomTheme!.css[next.scheme]['--ccm-accent'])

    const before = next.random!.seed
    next.reshuffleRandom()
    expect(next.random!.seed).not.toBe(before)
    next.holdRandom()
    expect(next.random!.held).toBe(true)
    expect(next.random!.pins[0]).toBe(next.random!.seed)
    const kept = next.random!.seed
    await nextTick()

    setActivePinia(createPinia())
    const third = useUiGenStore()
    expect(third.random!.seed).toBe(kept)
    third.dropRandom(kept)
    expect(third.random!.held).toBe(false)
    expect(third.random!.pins).toEqual([])

    third.setNextTheme('mist')
    expect(root.style.getPropertyValue('--ccm-accent')).toBe('')
    expect(root.style.getPropertyValue('--ccm-ansi-1')).toBe('')
  })
})
