import { nextPalette, nextSchemeFor, type NextScheme } from '@ccm/mist-tokens'
import { detectLocale, type Locale } from '../i18n'
import type { LanguagePref, ThemePref } from '../storage/localPrefs'

export function resolveLocale(pref: LanguagePref, languages: readonly string[]): Locale {
  return pref === 'vi' || pref === 'en' ? pref : detectLocale(languages)
}

export interface AppearanceInput {
  theme: ThemePref
  palette: string
  reduceTransparency: boolean
}

export interface AppearanceEnv {
  systemDark: boolean
  reducedTransparency: boolean
  reducedMotion: boolean
}

export interface ResolvedAppearance {
  palette: string
  scheme: NextScheme
  glass: 'full' | 'reduced'
  motion: 'full' | 'reduced'
  canvas: string
}

export function resolveAppearance(input: AppearanceInput, env: AppearanceEnv): ResolvedAppearance {
  const preferred: NextScheme = input.theme === 'auto' ? (env.systemDark ? 'dark' : 'light') : input.theme
  const palette = nextPalette(input.palette)
  const scheme = nextSchemeFor(palette.id, preferred)
  return {
    palette: palette.id,
    scheme,
    glass: input.reduceTransparency || env.reducedTransparency ? 'reduced' : 'full',
    motion: env.reducedMotion ? 'reduced' : 'full',
    canvas: palette.preview[scheme]?.canvas ?? (scheme === 'dark' ? '#0f171d' : '#eef3f6'),
  }
}

export function applyAppearance(resolved: ResolvedAppearance, root: HTMLElement = document.documentElement): void {
  root.dataset.ui = 'next'
  root.dataset.nextTheme = resolved.palette
  root.dataset.glass = resolved.glass
  root.dataset.motion = resolved.motion
  root.classList.toggle('dark', resolved.scheme === 'dark')
  for (const meta of root.ownerDocument.querySelectorAll<HTMLMetaElement>('meta[name="theme-color"]')) meta.content = resolved.canvas
}

export function readEnv(win: Pick<Window, 'matchMedia'> = window): AppearanceEnv {
  const q = (query: string): boolean => win.matchMedia?.(query).matches ?? false
  return {
    systemDark: q('(prefers-color-scheme: dark)'),
    reducedTransparency: q('(prefers-reduced-transparency: reduce)'),
    reducedMotion: q('(prefers-reduced-motion: reduce)'),
  }
}

export function startAppearance(read: () => AppearanceInput, root: HTMLElement = document.documentElement, win: Window = window): { refresh: () => void; stop: () => void } {
  let first = true
  let timer: ReturnType<typeof setTimeout> | null = null
  const refresh = (): void => {
    if (!first) {
      root.dataset.fade = '1'
      if (timer) clearTimeout(timer)
      timer = setTimeout(() => {
        timer = null
        delete root.dataset.fade
      }, 420)
    }
    first = false
    applyAppearance(resolveAppearance(read(), readEnv(win)), root)
  }
  const queries = ['(prefers-color-scheme: dark)', '(prefers-reduced-transparency: reduce)', '(prefers-reduced-motion: reduce)']
    .map((q) => win.matchMedia?.(q))
    .filter((m): m is MediaQueryList => Boolean(m))
  for (const m of queries) m.addEventListener?.('change', refresh)
  refresh()
  return {
    refresh,
    stop: () => {
      for (const m of queries) m.removeEventListener?.('change', refresh)
      if (timer) clearTimeout(timer)
    },
  }
}
