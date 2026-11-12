import { onBeforeUnmount, onMounted, ref, type Ref } from 'vue'

export function useNow(intervalMs = 20_000): Ref<number> {
  const now = ref(Date.now())
  let timer: ReturnType<typeof setInterval> | null = null
  onMounted(() => {
    timer = setInterval(() => {
      now.value = Date.now()
    }, intervalMs)
  })
  onBeforeUnmount(() => {
    if (timer) clearInterval(timer)
  })
  return now
}
