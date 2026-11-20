import { describe, expect, it, vi } from 'vitest'
import { DEFAULT_LOCAL_PREFS, loadLocalPrefs, normalizeLocalPrefs, saveLocalPrefs } from '../storage/localPrefs'
import { applyAppearance, resolveAppearance, startAppearance, type AppearanceEnv } from './appearance'

const env = (patch: Partial<AppearanceEnv> = {}): AppearanceEnv => ({ systemDark: false, reducedTransparency: false, reducedMotion: false, ...patch })
const input = (patch: Partial<typeof DEFAULT_LOCAL_PREFS> = {}) => ({ theme: 'auto' as const, palette: 'mist', reduceTransparency: false, ...patch })

describe('resolveAppearance', () => {
  it('follows the device in auto and the pick otherwise', () => {
    expect(resolveAppearance(input(), env()).scheme).toBe('light')
    expect(resolveAppearance(input(), env({ systemDark: true })).scheme).toBe('dark')
    expect(resolveAppearance(input({ theme: 'dark' }), env()).scheme).toBe('dark')
    expect(resolveAppearance(input({ theme: 'light' }), env({ systemDark: true })).scheme).toBe('light')
  })

  it('dark only palettes stay dark whatever the mode', () => {
    for (const palette of ['galaxy']) expect(resolveAppearance(input({ palette, theme: 'light' }), env()).scheme).toBe('dark')
  })

  it('falls back to the default palette for unknown ids', () => {
    expect(resolveAppearance(input({ palette: 'nope' }), env()).palette).toBe('mist')
  })

  it('reduces glass when the pref or the system asks for it', () => {
    expect(resolveAppearance(input(), env()).glass).toBe('full')
    expect(resolveAppearance(input({ reduceTransparency: true }), env()).glass).toBe('reduced')
    expect(resolveAppearance(input(), env({ reducedTransparency: true })).glass).toBe('reduced')
  })

  it('reduces motion from the system and gives the canvas for theme-color', () => {
    expect(resolveAppearance(input(), env({ reducedMotion: true })).motion).toBe('reduced')
    expect(resolveAppearance(input(), env()).canvas).toBe('#EEF3F6')
    expect(resolveAppearance(input({ theme: 'dark' }), env()).canvas).toBe('#0F171D')
  })
})

describe('applyAppearance', () => {
  it('writes attributes, the dark class and theme-color', () => {
    document.head.innerHTML = '<meta name="theme-color" content="#fff" media="(prefers-color-scheme: light)"><meta name="theme-color" content="#000">'
    const root = document.documentElement
    applyAppearance(resolveAppearance(input({ palette: 'galaxy', reduceTransparency: true }), env()), root)
    expect(root.dataset.ui).toBe('next')
    expect(root.dataset.nextTheme).toBe('galaxy')
    expect(root.dataset.glass).toBe('reduced')
    expect(root.classList.contains('dark')).toBe(true)
    for (const meta of document.querySelectorAll<HTMLMetaElement>('meta[name="theme-color"]')) expect(meta.content).toBe('#000000')
    applyAppearance(resolveAppearance(input(), env()), root)
    expect(root.classList.contains('dark')).toBe(false)
    expect(root.dataset.nextTheme).toBe('mist')
  })
})

describe('startAppearance', () => {
  function fakeWindow() {
    const listeners = new Map<string, Set<() => void>>()
    const state: Record<string, boolean> = {}
    const win = {
      matchMedia: (query: string) => ({
        get matches() {
          return state[query] ?? false
        },
        addEventListener: (_: string, fn: () => void) => (listeners.get(query) ?? listeners.set(query, new Set()).get(query)!).add(fn),
        removeEventListener: (_: string, fn: () => void) => listeners.get(query)?.delete(fn),
      }),
    } as unknown as Window
    return { win, set: (query: string, on: boolean) => ((state[query] = on), listeners.get(query)?.forEach((fn) => fn())) }
  }

  it('applies at once, re-applies on system changes and marks a fade only after the first paint', () => {
    vi.useFakeTimers()
    const root = document.createElement('html')
    const { win, set } = fakeWindow()
    const handle = startAppearance(() => input(), root, win)
    expect(root.classList.contains('dark')).toBe(false)
    expect(root.dataset.fade).toBeUndefined()
    set('(prefers-color-scheme: dark)', true)
    expect(root.classList.contains('dark')).toBe(true)
    expect(root.dataset.fade).toBe('1')
    vi.advanceTimersByTime(500)
    expect(root.dataset.fade).toBeUndefined()
    set('(prefers-reduced-transparency: reduce)', true)
    expect(root.dataset.glass).toBe('reduced')
    handle.stop()
    vi.useRealTimers()
  })
})

describe('local prefs migration', () => {
  it('an old blob without palette keeps its theme and gets Mist with glass on', () => {
    const prefs = normalizeLocalPrefs({ language: 'vi', theme: 'dark', monitorMode: 'groups' })
    expect(prefs.theme).toBe('dark')
    expect(prefs.language).toBe('vi')
    expect(prefs.palette).toBe('mist')
    expect(prefs.reduceTransparency).toBe(false)
  })

  it('keeps a known palette and rejects custom, unknown and non boolean values', () => {
    expect(normalizeLocalPrefs({ palette: 'galaxy' }).palette).toBe('galaxy')
    expect(normalizeLocalPrefs({ palette: 'custom' }).palette).toBe('mist')
    expect(normalizeLocalPrefs({ palette: 5 }).palette).toBe('mist')
    expect(normalizeLocalPrefs({ reduceTransparency: 'yes' }).reduceTransparency).toBe(false)
    expect(normalizeLocalPrefs({ reduceTransparency: true }).reduceTransparency).toBe(true)
  })

  it('round trips through storage', () => {
    const store = new Map<string, string>()
    const storage = { getItem: (k: string) => store.get(k) ?? null, setItem: (k: string, v: string) => void store.set(k, v) }
    saveLocalPrefs({ ...DEFAULT_LOCAL_PREFS, palette: 'mono', reduceTransparency: true, theme: 'light' }, storage)
    expect(loadLocalPrefs(storage)).toMatchObject({ palette: 'mono', reduceTransparency: true, theme: 'light' })
  })
})
