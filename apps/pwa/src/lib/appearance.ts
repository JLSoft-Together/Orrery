import { detectLocale, type Locale } from '../i18n'
import type { LanguagePref, ThemePref } from '../storage/localPrefs'

export function resolveLocale(pref: LanguagePref, languages: readonly string[]): Locale {
  return pref === 'vi' || pref === 'en' ? pref : detectLocale(languages)
}

export function applyTheme(theme: ThemePref, root: HTMLElement = document.documentElement): void {
  if (theme === 'auto') root.removeAttribute('data-theme')
  else root.setAttribute('data-theme', theme)
}
