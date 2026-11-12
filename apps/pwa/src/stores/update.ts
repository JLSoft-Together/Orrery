import { defineStore } from 'pinia'
import { ref } from 'vue'
import { UpdateChecker, type UpdateState, type VersionInfo } from '../update/version'
import type { SwHandle } from '../update/serviceWorker'

export const useUpdateStore = defineStore('update', () => {
  const state = ref<UpdateState>('current')
  const info = ref<VersionInfo | null>(null)
  const workerWaiting = ref(false)
  let checker: UpdateChecker | null = null
  let sw: SwHandle | null = null

  function configure(next: UpdateChecker, handle: SwHandle | null): void {
    checker = next
    sw = handle
  }

  async function check(): Promise<void> {
    void sw?.refresh()
    const result = await checker?.check()
    if (!result) return
    info.value = result.info
    state.value = result.state
  }

  function markWaiting(): void {
    workerWaiting.value = true
  }

  function reload(): void {
    if (sw) sw.apply()
    else location.reload()
  }

  return { state, info, workerWaiting, configure, check, markWaiting, reload }
})
