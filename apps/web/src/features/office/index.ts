import { defineAsyncComponent, watch } from 'vue'
import type { Pinia } from 'pinia'
import type { OfficeSnapshot } from '@ccm/habblaud/shared/types'
import { i18n } from '../../i18n'
import { registerEventHandler } from '../../core/stores/connectionEvents'
import { useSettingsStore } from '../../core/stores/settings'

let i18nPending: Promise<void> | null = null

export function loadOfficeI18n(): Promise<void> {
  i18nPending ??= Promise.all([import('./i18n/office-v60'), import('./i18n/office-v62'), import('./i18n/office-v63')])
    .then((mods) => {
      for (const m of mods) {
        i18n.global.mergeLocaleMessage('en', m.default.en)
        i18n.global.mergeLocaleMessage('vi', m.default.vi)
      }
    })
    .catch(() => {
      i18nPending = null
    })
  return i18nPending
}

export const OfficeEntry = defineAsyncComponent(async () => {
  await loadOfficeI18n()
  return (await import('./components/OfficeView.vue')).default
})

export const OfficeToolbar = defineAsyncComponent(async () => {
  await loadOfficeI18n()
  return (await import('./components/OfficeToolbar.vue')).default
})

let registered = false

export function registerOfficeFeed(pinia?: Pinia): void {
  if (registered) return
  registered = true
  let feedPending: Promise<typeof import('./lib/feed')['officeFeed']> | null = null
  const feed = () => (feedPending ??= import('./lib/feed').then((m) => m.officeFeed))
  registerEventHandler('office.snapshot', (payload) => {
    void feed().then((f) => f.ingestLive(payload as OfficeSnapshot))
  })
  const settings = useSettingsStore(pinia)
  watch(
    () => settings.officeEnabled,
    (on) => {
      if (on) void loadOfficeI18n()
    },
    { immediate: true },
  )
}
