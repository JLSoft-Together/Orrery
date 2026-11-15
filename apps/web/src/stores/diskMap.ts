import { defineStore } from 'pinia'
import { computed, ref, shallowRef } from 'vue'
import {
  diskMapCancel,
  diskMapChildren,
  diskMapDelete,
  diskMapHistory,
  diskMapPause,
  diskMapResume,
  diskMapReveal,
  diskMapScan,
  diskMapStatus,
  diskMapVolumes,
  hasDiskMap,
  isUnder,
  loadChart,
  loadLevel,
  onDiskMapDone,
  onDiskMapProgress,
  parentPath,
  saveChart,
  saveLevel,
  type DiskChart,
  type DiskChildren,
  type DiskDeleteResult,
  type DiskEntry,
  type DiskHistoryEntry,
  type DiskLevel,
  type DiskProgress,
  type DiskStatus,
  type DiskVolume,
} from '../lib/diskMap'

export interface DeleteReport {
  freed: number
  done: number
  failed: { path: string; reason: string }[]
  permanent: boolean
}

export const useDiskMapStore = defineStore('diskMap', () => {
  const volumes = shallowRef<DiskVolume[]>([])
  const status = shallowRef<DiskStatus | null>(null)
  const progress = shallowRef<DiskProgress | null>(null)
  const path = ref('')
  const children = shallowRef<DiskChildren | null>(null)
  const loading = ref(false)
  const level = ref<DiskLevel>(loadLevel())
  const chart = ref<DiskChart>(loadChart())
  const roots = ref<string[]>([])
  const selected = ref<Set<string>>(new Set())
  const anchor = ref<string | null>(null)
  const deleting = ref(false)
  const report = shallowRef<DeleteReport | null>(null)
  const history = shallowRef<DiskHistoryEntry[]>([])
  const sub = shallowRef<Map<string, DiskEntry[]>>(new Map())
  const supported = hasDiskMap
  let started = false
  let offs: (() => void)[] = []
  let navToken = 0

  const items = computed(() => children.value?.items ?? [])
  const scanning = computed(() => !!status.value?.scanning)
  const paused = computed(() => !!status.value?.paused)
  const indexed = computed(() => !!status.value?.indexed)
  const canUp = computed(() => {
    if (!path.value) return false
    const p = parentPath(path.value)
    if (!p) return false
    const rs = status.value?.roots ?? []
    return rs.length === 0 || rs.some((r) => isUnder(p, r))
  })
  const selectedItems = computed(() => items.value.filter((i) => selected.value.has(i.path)))
  const selectedSize = computed(() => selectedItems.value.reduce((s, i) => s + i.size, 0))
  const totalFreed = computed(() => history.value.reduce((s, h) => s + h.freed, 0))

  async function refreshStatus(): Promise<void> {
    const s = await diskMapStatus()
    if (s) status.value = s
  }

  async function loadVolumes(): Promise<void> {
    volumes.value = await diskMapVolumes()
  }

  async function loadHistory(): Promise<void> {
    history.value = await diskMapHistory(100)
  }

  async function open(target: string): Promise<void> {
    const token = ++navToken
    path.value = target
    selected.value = new Set()
    anchor.value = null
    sub.value = new Map()
    loading.value = true
    const res = await diskMapChildren(target, 2000)
    if (token !== navToken) return
    children.value = res
    loading.value = false
  }

  async function reload(): Promise<void> {
    if (path.value) await open(path.value)
  }

  async function init(): Promise<void> {
    if (!hasDiskMap) return
    if (!started) {
      started = true
      offs.push(
        onDiskMapProgress((p) => {
          progress.value = p
        }),
        onDiskMapDone(() => {
          progress.value = null
          void refreshStatus().then(async () => {
            await loadVolumes()
            if (!path.value && status.value?.roots[0]) await open(status.value.roots[0])
            else await reload()
          })
        }),
      )
    }
    await Promise.all([loadVolumes(), refreshStatus(), loadHistory()])
    if (roots.value.length === 0 && status.value?.roots.length) roots.value = [...status.value.roots]
    if (!path.value && status.value?.indexed && status.value.roots[0]) await open(status.value.roots[0])
  }

  function dispose(): void {
    for (const off of offs) off()
    offs = []
    started = false
  }

  function toggleRoot(mount: string): void {
    const next = new Set(roots.value)
    if (next.has(mount)) next.delete(mount)
    else next.add(mount)
    roots.value = [...next]
  }

  async function scan(): Promise<boolean> {
    if (!roots.value.length || scanning.value) return false
    progress.value = { files: 0, bytes: 0, current: '', elapsedMs: 0 }
    const ok = await diskMapScan(roots.value, level.value)
    if (!ok) {
      progress.value = null
      return false
    }
    children.value = null
    path.value = ''
    selected.value = new Set()
    await refreshStatus()
    return true
  }

  async function pause(): Promise<void> {
    if (await diskMapPause()) await refreshStatus()
  }

  async function resume(): Promise<void> {
    if (await diskMapResume()) await refreshStatus()
  }

  async function cancel(): Promise<void> {
    await diskMapCancel()
    progress.value = null
    await refreshStatus()
  }

  async function up(): Promise<void> {
    const p = path.value ? parentPath(path.value) : null
    if (p && canUp.value) await open(p)
  }

  function setLevel(next: DiskLevel): void {
    level.value = next
    saveLevel(next)
  }

  function setChart(next: DiskChart): void {
    chart.value = next
    saveChart(next)
  }

  function toggle(p: string, range = false): void {
    const next = new Set(selected.value)
    if (range && anchor.value) {
      const list = items.value
      const a = list.findIndex((i) => i.path === anchor.value)
      const b = list.findIndex((i) => i.path === p)
      if (a >= 0 && b >= 0) {
        const [lo, hi] = a < b ? [a, b] : [b, a]
        for (let i = lo; i <= hi; i++) next.add(list[i]!.path)
        selected.value = next
        return
      }
    }
    if (next.has(p)) next.delete(p)
    else next.add(p)
    anchor.value = p
    selected.value = next
  }

  function selectOnly(p: string): void {
    selected.value = new Set([p])
    anchor.value = p
  }

  function clearSelection(): void {
    selected.value = new Set()
    anchor.value = null
  }

  function selectAll(): void {
    selected.value = new Set(items.value.map((i) => i.path))
  }

  async function remove(paths: string[], permanent: boolean): Promise<DiskDeleteResult | null> {
    if (!paths.length || deleting.value) return null
    deleting.value = true
    report.value = null
    const res = await diskMapDelete(paths, permanent)
    deleting.value = false
    if (!res) {
      report.value = { freed: 0, done: 0, failed: paths.map((p) => ({ path: p, reason: 'unavailable' })), permanent }
      return null
    }
    if (res.cancelled) return res
    const gone = new Set(res.done.map((d) => d.path))
    const next = new Set(selected.value)
    for (const g of gone) next.delete(g)
    selected.value = next
    report.value = { freed: res.freed, done: res.done.length, failed: res.failed, permanent: res.permanent }
    const keep = new Set(res.failed.map((f) => f.path))
    await Promise.all([reload(), loadVolumes(), loadHistory(), refreshStatus()])
    selected.value = new Set(items.value.filter((i) => keep.has(i.path)).map((i) => i.path))
    return res
  }

  async function removeSelected(permanent: boolean): Promise<DiskDeleteResult | null> {
    return remove([...selected.value], permanent)
  }

  function dismissReport(): void {
    report.value = null
  }

  function reveal(p: string): Promise<void> {
    return diskMapReveal(p)
  }

  async function loadSub(dirs: string[]): Promise<void> {
    const token = navToken
    const want = dirs.filter((d) => !sub.value.has(d))
    const results = await Promise.all(want.map((d) => diskMapChildren(d, 40)))
    if (token !== navToken) return
    const map = new Map(sub.value)
    want.forEach((d, i) => map.set(d, results[i]?.items ?? []))
    sub.value = map
  }

  return {
    volumes,
    status,
    progress,
    path,
    children,
    loading,
    level,
    chart,
    roots,
    selected,
    deleting,
    report,
    history,
    sub,
    supported,
    items,
    scanning,
    paused,
    indexed,
    canUp,
    selectedItems,
    selectedSize,
    totalFreed,
    init,
    dispose,
    refreshStatus,
    loadVolumes,
    loadHistory,
    open,
    reload,
    toggleRoot,
    scan,
    pause,
    resume,
    cancel,
    up,
    setLevel,
    setChart,
    toggle,
    selectOnly,
    clearSelection,
    selectAll,
    remove,
    removeSelected,
    dismissReport,
    reveal,
    loadSub,
  }
})
