import { computed, ref, shallowRef } from 'vue'
import { defineStore } from 'pinia'
import type { DiskDeleteResult } from '@ccm/shared'
import {
  diskAppsDelete,
  diskAppsLeftovers,
  diskAppsList,
  diskAppsSettings,
  diskAppsUninstall,
  filterApps,
  hasDiskApps,
  type AppLeftoversResult,
  type AppSort,
  type AppUninstallResult,
  type InstalledApp,
  type InstalledAppsResult,
} from '../lib/diskApps'
import { useDiskMapStore } from './diskMap'

export const useDiskAppsStore = defineStore('diskApps', () => {
  const supported = hasDiskApps
  const active = ref(false)
  const result = shallowRef<InstalledAppsResult | null>(null)
  const loading = ref(false)
  const query = ref('')
  const sort = ref<AppSort>('size')
  const busy = ref<string | null>(null)
  const notice = shallowRef<{ id: string; name: string; res: AppUninstallResult } | null>(null)
  const leftovers = shallowRef<AppLeftoversResult | null>(null)
  const scanning = ref<string | null>(null)
  const picked = ref<Set<string>>(new Set())
  const deleted = shallowRef<DiskDeleteResult | null>(null)

  const apps = computed(() => result.value?.apps ?? [])
  const removed = computed(() => result.value?.removed ?? [])
  const shown = computed(() => filterApps(apps.value, query.value, sort.value))
  const totalBytes = computed(() => apps.value.reduce((s, a) => s + (a.size ?? 0), 0))
  const pickedBytes = computed(() => (leftovers.value?.items ?? []).filter((i) => picked.value.has(i.path)).reduce((s, i) => s + (i.size ?? 0), 0))
  const leftoverApp = computed<InstalledApp | null>(() => {
    const id = leftovers.value?.id ?? scanning.value
    return id ? (apps.value.find((a) => a.id === id) ?? removed.value.find((a) => a.id === id) ?? null) : null
  })

  async function load(): Promise<void> {
    if (loading.value) return
    loading.value = true
    try {
      result.value = await diskAppsList()
    } finally {
      loading.value = false
    }
  }

  function open(): void {
    active.value = true
    if (!result.value) void load()
  }

  async function uninstall(app: InstalledApp): Promise<void> {
    if (busy.value) return
    busy.value = app.id
    notice.value = null
    try {
      const res = await diskAppsUninstall(app.id)
      notice.value = { id: app.id, name: app.name, res }
      if (res.ok && res.mode === 'trashed') await load()
    } finally {
      busy.value = null
    }
  }

  async function scanLeftovers(app: InstalledApp): Promise<void> {
    scanning.value = app.id
    leftovers.value = null
    deleted.value = null
    picked.value = new Set()
    try {
      leftovers.value = (await diskAppsLeftovers(app.id)) ?? { id: app.id, installed: true, items: [] }
    } finally {
      scanning.value = null
    }
  }

  function closeLeftovers(): void {
    leftovers.value = null
    deleted.value = null
    picked.value = new Set()
  }

  function toggle(path: string): void {
    const next = new Set(picked.value)
    if (next.has(path)) next.delete(path)
    else next.add(path)
    picked.value = next
  }

  async function removeLeftovers(permanent: boolean): Promise<void> {
    const cur = leftovers.value
    if (!cur || !picked.value.size || busy.value) return
    busy.value = cur.id
    try {
      const res = await diskAppsDelete(cur.id, [...picked.value], permanent)
      deleted.value = res
      if (!res.cancelled) {
        const gone = new Set(res.done.map((d) => d.path.toLowerCase()))
        leftovers.value = { ...cur, items: cur.items.filter((i) => !gone.has(i.path.toLowerCase())) }
        picked.value = new Set([...picked.value].filter((p) => !gone.has(p.toLowerCase())))
      }
      if (res.done.length) void useDiskMapStore().loadHistory()
    } finally {
      busy.value = null
    }
  }

  const openSettings = (): Promise<boolean> => diskAppsSettings()

  return { supported, active, result, loading, query, sort, busy, notice, leftovers, scanning, picked, deleted, apps, removed, shown, totalBytes, pickedBytes, leftoverApp, load, open, uninstall, scanLeftovers, closeLeftovers, toggle, removeLeftovers, openSettings }
})
