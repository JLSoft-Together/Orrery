import { computed, onBeforeUnmount, ref, type ComputedRef } from 'vue'
import type { NextScheme } from '@ccm/mist-tokens'
import { usePrefsStore } from '../stores/prefs'
import { resolveAppearance } from './appearance'

export function useScheme(): ComputedRef<NextScheme> {
  const prefs = usePrefsStore()
  const media = window.matchMedia('(prefers-color-scheme: dark)')
  const systemDark = ref(media.matches)
  const onChange = (event: MediaQueryListEvent): void => {
    systemDark.value = event.matches
  }
  media.addEventListener('change', onChange)
  onBeforeUnmount(() => media.removeEventListener('change', onChange))
  return computed(() => resolveAppearance(prefs.local, { systemDark: systemDark.value, reducedTransparency: false, reducedMotion: false }).scheme)
}
