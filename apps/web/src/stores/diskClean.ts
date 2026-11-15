import { computed, ref, shallowRef } from 'vue'
import { defineStore } from 'pinia'
import type { DiskDeleteResult } from '@ccm/shared'
import {
  clampDays,
  diskCleanCancel,
  diskCleanCategories,
  diskCleanDelete,
  diskCleanScan,
  hasDiskClean,
  inProject,
  loadDays,
  loadFolders,
  lockReason,
  onDiskCleanProgress,
  preselect,
  ON_DEMAND,
  PROJECT_IDS,
  saveDays,
  saveFolders,
  under,
  type CleanCategory,
  type CleanItem,
  type CleanProgress,
  type CleanScanResult,
} from '../lib/diskClean'

export interface CleanReport {
  done: number
  freed: number
  failed: DiskDeleteResult['failed']
  permanent: boolean
}

export const useDiskCleanStore = defineStore('diskClean', () => {
  const supported = hasDiskClean
  const active = ref(false)
  const categories = shallowRef<CleanCategory[]>([])
  const items = shallowRef<CleanItem[]>([])
  const progress = shallowRef<Record<string, CleanProgress>>({})
  const scanning = ref(false)
  const scanned = ref(false)
  const deleting = ref(false)
  const selected = ref<Set<string>>(new Set())
  const project = ref<string | null>(null)
  const days = ref(loadDays())
  const report = shallowRef<CleanReport | null>(null)
  const open = shallowRef<string[]>([])
  const folders = ref<string[]>(loadFolders())
  const measured = ref<Set<string>>(new Set())
  let off: (() => void) | null = null
  let seq = 0

  const visible = computed(() => items.value.filter((i) => inProject(i, project.value)))
  const byCategory = computed(() => {
    const map = new Map<string, CleanItem[]>()
    for (const i of visible.value) {
      const list = map.get(i.category) ?? []
      list.push(i)
      map.set(i.category, list)
    }
    for (const list of map.values()) list.sort((a, b) => b.size - a.size)
    return map
  })
  const selectedItems = computed(() => visible.value.filter((i) => selected.value.has(i.path) && !lockReason(i, open.value)))
  const selectedSize = computed(() => selectedItems.value.reduce((s, i) => s + i.size, 0))

  async function init(): Promise<void> {
    if (!supported) return
    if (!off) {
      off = onDiskCleanProgress((p) => {
        if (!scanning.value) return
        progress.value = { ...progress.value, [p.category]: p }
        if (p.state === 'items' && p.items?.length) items.value = [...items.value, ...p.items]
      })
    }
    if (!categories.value.length) categories.value = await diskCleanCategories()
  }

  function dispose(): void {
    off?.()
    off = null
  }

  function setOpen(list: string[]): void {
    open.value = [...list]
  }

  function idsFor(dir: string | null): string[] {
    const all = categories.value.map((c) => c.id).filter((id) => !ON_DEMAND.includes(id))
    return dir ? all.filter((id) => PROJECT_IDS.includes(id)) : all
  }

  async function run(ids: string[], roots: string[], openNow: string[], fresh: boolean): Promise<CleanScanResult | null> {
    if (!supported || scanning.value) return null
    await init()
    const token = ++seq
    const forProject = project.value
    setOpen(openNow)
    const kept = items.value.filter((i) => !ids.includes(i.category))
    scanning.value = true
    progress.value = {}
    items.value = kept
    if (fresh) selected.value = new Set()
    report.value = null
    try {
      const res = await diskCleanScan(ids, roots, openNow)
      if (token !== seq || forProject !== project.value) return null
      items.value = [...kept, ...res.items]
      const picked = preselect(res.items, categories.value, days.value, Date.now(), open.value)
      selected.value = fresh ? picked : new Set([...selected.value, ...picked])
      return res
    } finally {
      if (token === seq) scanning.value = false
    }
  }

  async function scan(roots: string[], openNow: string[]): Promise<void> {
    await init()
    if (await run(idsFor(project.value), project.value ? [project.value] : roots, openNow, true)) scanned.value = true
  }

  async function measure(id: string, openNow: string[]): Promise<void> {
    const res = await run([id], [], openNow, false)
    if (res && !res.cancelled) measured.value = new Set([...measured.value, id])
  }

  async function cancel(): Promise<void> {
    await diskCleanCancel()
  }

  function openFor(dir: string | null): void {
    active.value = true
    if (dir !== project.value) {
      if (scanning.value) void diskCleanCancel()
      project.value = dir
      scanned.value = false
      measured.value = new Set()
      items.value = []
      selected.value = new Set()
    }
  }

  function clearProject(): void {
    openFor(null)
  }

  function setDays(v: number): void {
    days.value = clampDays(v)
    saveDays(days.value)
  }

  function toggle(path: string): void {
    const item = items.value.find((i) => i.path === path)
    if (!item || lockReason(item, open.value)) return
    const next = new Set(selected.value)
    if (!next.delete(path)) next.add(path)
    selected.value = next
  }

  function toggleCategory(id: string): void {
    const list = (byCategory.value.get(id) ?? []).filter((i) => !lockReason(i, open.value))
    const allOn = list.length > 0 && list.every((i) => selected.value.has(i.path))
    const next = new Set(selected.value)
    for (const i of list) allOn ? next.delete(i.path) : next.add(i.path)
    selected.value = next
  }

  function clearSelection(): void {
    selected.value = new Set()
  }

  async function removeSelected(permanent: boolean, openNow: string[]): Promise<DiskDeleteResult | null> {
    if (deleting.value) return null
    setOpen(openNow)
    const targets = selectedItems.value.filter((i) => !lockReason(i, openNow))
    const dropped = [...selected.value].filter((p) => !targets.some((t) => t.path === p))
    if (dropped.length) {
      const next = new Set(selected.value)
      for (const p of dropped) {
        const it = items.value.find((i) => i.path === p)
        if (!it || lockReason(it, openNow)) next.delete(p)
      }
      selected.value = next
    }
    if (!targets.length) return null
    deleting.value = true
    try {
      const res = await diskCleanDelete(
        targets.map((t) => t.path),
        permanent,
        openNow,
      )
      if (!res) return null
      if (res.cancelled && !res.done.length && !res.failed.length) return res
      const gone = new Set(res.done.map((d) => d.path.toLowerCase()))
      for (const f of res.failed) if (f.reason === 'ENOENT') gone.add(f.path.toLowerCase())
      items.value = items.value.filter((i) => !gone.has(i.path.toLowerCase()))
      const next = new Set([...selected.value].filter((p) => !gone.has(p.toLowerCase())))
      selected.value = next
      report.value = { done: res.done.length, freed: res.freed, failed: res.failed, permanent: res.permanent }
      return res
    } finally {
      deleting.value = false
    }
  }

  function addFolder(dir: string): void {
    if (!dir || folders.value.some((f) => under(f, dir) && under(dir, f))) return
    folders.value = [...folders.value, dir]
    saveFolders(folders.value)
  }

  function removeFolder(dir: string): void {
    folders.value = folders.value.filter((f) => f !== dir)
    saveFolders(folders.value)
  }

  function dismissReport(): void {
    report.value = null
  }

  return {
    supported,
    active,
    categories,
    items,
    progress,
    scanning,
    scanned,
    deleting,
    selected,
    project,
    days,
    report,
    open,
    folders,
    measured,
    visible,
    byCategory,
    selectedItems,
    selectedSize,
    init,
    dispose,
    setOpen,
    scan,
    measure,
    cancel,
    openFor,
    clearProject,
    setDays,
    toggle,
    toggleCategory,
    clearSelection,
    removeSelected,
    addFolder,
    removeFolder,
    dismissReport,
  }
})
