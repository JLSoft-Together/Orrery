import { createI18n } from 'vue-i18n'
import { en } from './en'
import { vi } from './vi'

export type Locale = 'vi' | 'en'

export function detectLocale(languages: readonly string[]): Locale {
  for (const language of languages) {
    const tag = language.toLowerCase()
    if (tag.startsWith('vi')) return 'vi'
    if (tag.startsWith('en')) return 'en'
  }
  return 'en'
}

export function createAppI18n(locale: Locale) {
  return createI18n({
    legacy: false,
    locale,
    fallbackLocale: 'en',
    messages: { vi, en },
    missingWarn: false,
    fallbackWarn: false,
  })
}
