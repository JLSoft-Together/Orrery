import { shallowRef } from 'vue'
import type { PtyPreset } from '@ccm/shared'
import { i18n } from '../../i18n'
import { hasDeepSeek } from '../../lib/desktop'

type Store = typeof import('./store')

const loaded = shallowRef<Store | null>(null)
let pending: Promise<Store | null> | null = null

export function loadDeepSeek(): Promise<Store | null> {
  if (!hasDeepSeek) return Promise.resolve(null)
  if (loaded.value) return Promise.resolve(loaded.value)
  pending ??= Promise.all([import('./store'), import('./i18n')])
    .then(([store, messages]) => {
      i18n.global.mergeLocaleMessage('en', messages.default.en)
      i18n.global.mergeLocaleMessage('vi', messages.default.vi)
      loaded.value = store
      return store
    })
    .catch(() => {
      pending = null
      return null
    })
  return pending
}

export const deepseekStore = (): ReturnType<Store['useDeepSeekStore']> | null => loaded.value?.useDeepSeekStore() ?? null

export function gateDeepSeek(preset: PtyPreset): boolean {
  if (preset !== 'deepseek') return false
  const store = deepseekStore()
  if (store) return store.gate(preset)
  void loadDeepSeek().then((m) => m?.useDeepSeekStore().gate(preset))
  return true
}

export async function gateDeepSeekAgent(): Promise<boolean> {
  const m = await loadDeepSeek()
  return m ? m.useDeepSeekStore().gateAgent() : false
}

export function warmDeepSeek(delayMs: number | null): void {
  void loadDeepSeek().then((m) => {
    if (m && delayMs !== null) window.setTimeout(() => void m.useDeepSeekStore().refresh().catch(() => null), delayMs)
  })
}

export const loadSettingsDeepSeek = async () => {
  await loadDeepSeek()
  return (await import('./SettingsDeepSeek.vue')).default
}
