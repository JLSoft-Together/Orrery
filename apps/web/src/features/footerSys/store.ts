import { shallowRef } from 'vue'
import { defineStore } from 'pinia'
import { desktop } from '../../lib/desktop'
import { parseSample, sysChanged, SYS_POLL_MS, type SysSample } from './format'

interface SysBridge {
  sysSample?: () => Promise<unknown>
}

const bridge = desktop as unknown as SysBridge | undefined

export const hasSysSample = typeof bridge?.sysSample === 'function'

export const useSysStatsStore = defineStore('sysStats', () => {
  const sample = shallowRef<SysSample | null>(null)
  let watchers = 0
  let timer: ReturnType<typeof setTimeout> | null = null
  let busy = false

  const visible = (): boolean => typeof document === 'undefined' || document.visibilityState === 'visible'

  async function refresh(): Promise<void> {
    if (busy || !bridge?.sysSample) return
    busy = true
    try {
      const next = parseSample(await bridge.sysSample())
      if (next && sysChanged(sample.value, next)) sample.value = next
    } catch {
      return
    } finally {
      busy = false
    }
  }

  function clear(): void {
    if (timer) clearTimeout(timer)
    timer = null
  }

  function tick(): void {
    timer = null
    if (watchers <= 0 || !visible()) return
    void refresh().finally(() => {
      if (watchers > 0 && visible() && !timer) timer = setTimeout(tick, SYS_POLL_MS)
    })
  }

  function onVisibility(): void {
    clear()
    if (watchers > 0 && visible()) tick()
  }

  function start(): void {
    if (!hasSysSample) return
    watchers++
    if (watchers === 1) {
      document.addEventListener('visibilitychange', onVisibility)
      if (!timer) tick()
    }
  }

  function stop(): void {
    if (watchers === 0) return
    watchers--
    if (watchers === 0) {
      clear()
      document.removeEventListener('visibilitychange', onVisibility)
      sample.value = null
    }
  }

  return { sample, start, stop, refresh }
})
