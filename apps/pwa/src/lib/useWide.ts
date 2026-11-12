import { onBeforeUnmount, ref, type Ref } from 'vue'

export function useWide(query = '(min-width: 768px)'): Ref<boolean> {
  const media = window.matchMedia(query)
  const wide = ref(media.matches)
  const onChange = (event: MediaQueryListEvent): void => {
    wide.value = event.matches
  }
  media.addEventListener('change', onChange)
  onBeforeUnmount(() => media.removeEventListener('change', onChange))
  return wide
}
