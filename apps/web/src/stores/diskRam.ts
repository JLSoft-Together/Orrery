import { computed, ref, shallowRef } from 'vue'
import { defineStore } from 'pinia'
import { diskRamKill, diskRamSnapshot, groupRam, hasDiskRam, RAM_POLL_MS, type RamKillResult, type RamKillTarget, type RamSnapshot } from '../lib/diskApps'

export type RamView = 'apps' | 'procs'

export const useDiskRamStore = defineStore('diskRam', () => {
  const supported = hasDiskRam
  const active = ref(false)
  const snap = shallowRef<RamSnapshot | null>(null)
  const view = ref<RamView>('apps')
  const query = ref('')
  const killing = ref<string | null>(null)
  const notice = shallowRef<{ name: string; res: RamKillResult } | null>(null)
  let timer: ReturnType<typeof setTimeout> | null = null
  let watchers = 0

  const groups = computed(() => groupRam(snap.value?.procs ?? []))
  const usedMem = computed(() => (snap.value ? snap.value.totalMem - snap.value.freeMem : 0))

  async function refresh(): Promise<void> {
    const next = await diskRamSnapshot()
    if (next) snap.value = next
  }

  function tick(): void {
    timer = null
    if (watchers <= 0) return
    const run = typeof document !== 'undefined' && document.hidden ? Promise.resolve() : refresh()
    void run.finally(() => {
      if (watchers > 0 && !timer) timer = setTimeout(tick, RAM_POLL_MS)
    })
  }

  function start(): void {
    watchers++
    if (watchers === 1 && !timer) tick()
  }

  function stop(): void {
    watchers = Math.max(0, watchers - 1)
    if (watchers === 0 && timer) {
      clearTimeout(timer)
      timer = null
    }
  }

  async function kill(key: string, name: string, targets: RamKillTarget[]): Promise<void> {
    if (killing.value || !targets.length) return
    killing.value = key
    notice.value = null
    try {
      const res = await diskRamKill(targets)
      if (res.error !== 'cancelled') notice.value = { name, res }
      await refresh()
    } finally {
      killing.value = null
    }
  }

  return { supported, active, snap, view, query, killing, notice, groups, usedMem, refresh, start, stop, kill }
})
