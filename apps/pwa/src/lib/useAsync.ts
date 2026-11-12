import { onBeforeUnmount, ref, shallowRef, type Ref, type ShallowRef } from 'vue'
import { ViewerError } from './viewerApi'

export interface Async<T> {
  data: ShallowRef<T | null>
  error: Ref<string | null>
  loading: Ref<boolean>
  reload: () => Promise<void>
}

export function useAsync<T>(load: () => Promise<T>, immediate = true): Async<T> {
  const data = shallowRef<T | null>(null)
  const error = ref<string | null>(null)
  const loading = ref(false)
  let generation = 0
  let alive = true
  onBeforeUnmount(() => {
    alive = false
  })

  async function reload(): Promise<void> {
    const mine = ++generation
    loading.value = true
    try {
      const next = await load()
      if (!alive || mine !== generation) return
      data.value = next
      error.value = null
    } catch (e) {
      if (!alive || mine !== generation) return
      error.value = e instanceof ViewerError ? e.code : 'internal'
    } finally {
      if (alive && mine === generation) loading.value = false
    }
  }

  if (immediate) void reload()
  return { data, error, loading, reload }
}
