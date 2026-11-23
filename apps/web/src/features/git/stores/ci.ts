import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import type { GhCiState } from '@ccm/shared'
import { dirKey } from '../../../core/stores/favorites'
import { ghCi, hasGhPr } from '../../../core/lib/preview'
import { chipState, type CiEntry } from '../lib/prView'

export const useCiStore = defineStore('ci', () => {
  const byRoot = ref<Record<string, CiEntry>>({})
  const clock = ref(Date.now())
  let tick: ReturnType<typeof setInterval> | null = null

  function set(root: string, branch: string, state: GhCiState, at = Date.now()): void {
    const key = dirKey(root)
    const prev = byRoot.value[key]
    if (prev && prev.state === state && prev.branch === branch) return
    clock.value = Date.now()
    byRoot.value = { ...byRoot.value, [key]: { state, at, branch } }
    arm()
  }

  function clear(root: string): void {
    const key = dirKey(root)
    if (!(key in byRoot.value)) return
    const next = { ...byRoot.value }
    delete next[key]
    byRoot.value = next
  }

  function arm(): void {
    if (tick || typeof window === 'undefined') return
    tick = setInterval(() => {
      clock.value = Date.now()
      const live = Object.values(byRoot.value).some((e) => e.state === 'success' && clock.value - e.at < 10 * 60_000 + 1000)
      if (!live && tick) {
        clearInterval(tick)
        tick = null
      }
    }, 15_000)
  }

  function chip(root: string): 'pending' | 'success' | 'failure' | null {
    return chipState(byRoot.value[dirKey(root)], clock.value)
  }

  function entry(root: string): CiEntry | undefined {
    return byRoot.value[dirKey(root)]
  }

  async function loadWatches(): Promise<void> {
    if (!hasGhPr) return
    const res = await ghCi({ op: 'list' })
    if (!res.ok) return
    for (const w of res.watches) set(w.root, w.branch, 'pending', w.since)
  }

  const any = computed(() => Object.keys(byRoot.value).length > 0)

  return { byRoot, clock, any, set, clear, chip, entry, loadWatches }
})
