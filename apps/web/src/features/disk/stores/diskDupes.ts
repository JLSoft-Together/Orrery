import { defineStore } from 'pinia'
import { computed, ref, shallowRef, triggerRef } from 'vue'
import {
  diskMapDelete,
  diskMapDupes,
  diskMapDupesCancel,
  diskMapPause,
  diskMapResume,
  diskMapReveal,
  hasDiskDupes,
  loadLevel,
  onDiskMapDupesDone,
  onDiskMapDupesProgress,
  type DiskDupeGroup,
  type DiskDupesDone,
  type DiskDupesProgress,
} from '../lib/diskMap'
import { defaultTicked, filterGroups, groupKey, keepOne, removePaths, tickedPaths, toggleTick, totals, wastedOf, type DupeFilters } from '../components/dupes/model'
import { useDiskMapStore } from './diskMap'

export const DUPE_MIN_SIZES = [1, 10, 100, 1024] as const
export const DEFAULT_MIN_MB = 1
export const DELETE_BATCH = 5000

export type DupeError = 'unavailable' | 'busy' | 'not-indexed' | 'invalid' | 'failed' | 'exited'

export interface DupeReport {
  freed: number
  done: number
  failed: { path: string; reason: string }[]
  permanent: boolean
}

export function dupeError(code: string | undefined): DupeError {
  if (code === 'busy' || code === 'not-indexed' || code === 'invalid' || code === 'unavailable') return code
  return 'failed'
}

export const useDiskDupesStore = defineStore('diskDupes', () => {
  const supported = hasDiskDupes
  const active = ref(false)
  const running = ref(false)
  const paused = ref(false)
  const progress = shallowRef<DiskDupesProgress | null>(null)
  const result = shallowRef<DiskDupesDone | null>(null)
  const groups = shallowRef<DiskDupeGroup[]>([])
  const ticked = shallowRef<Set<string>>(new Set())
  const expanded = shallowRef<Set<string>>(new Set())
  const scope = ref('')
  const minMb = ref(DEFAULT_MIN_MB)
  const filters = ref<DupeFilters>({ folder: '', kind: '', minSize: 0 })
  const error = ref<DupeError | null>(null)
  const detail = ref('')
  const deleting = ref(false)
  const report = shallowRef<DupeReport | null>(null)
  let offs: (() => void)[] = []
  let started = false

  const shown = computed(() => filterGroups(groups.value, filters.value))
  const plan = computed(() => totals(shown.value, ticked.value))
  const wasted = computed(() => wastedOf(groups.value))
  const shownWasted = computed(() => wastedOf(shown.value))
  const hasResult = computed(() => result.value !== null && result.value.ok)

  function init(): void {
    if (!hasDiskDupes || started) return
    started = true
    offs.push(
      onDiskMapDupesProgress((p) => {
        progress.value = p
        paused.value = p.paused
      }),
      onDiskMapDupesDone((d) => finish(d)),
    )
  }

  function dispose(): void {
    for (const off of offs) off()
    offs = []
    started = false
  }

  function finish(d: DiskDupesDone | null): void {
    if (!running.value && !d?.ok) return
    running.value = false
    paused.value = false
    progress.value = null
    if (!d) {
      error.value = 'failed'
      return
    }
    if (d.cancelled) return
    if (!d.ok) {
      error.value = d.error === 'exited' ? 'exited' : 'failed'
      detail.value = d.error ?? ''
      return
    }
    result.value = d
    groups.value = d.groups
    ticked.value = defaultTicked(d.groups)
    expanded.value = new Set()
    report.value = null
  }

  async function start(): Promise<boolean> {
    if (!hasDiskDupes || running.value) return false
    error.value = null
    detail.value = ''
    report.value = null
    running.value = true
    paused.value = false
    progress.value = null
    const req = { minSize: Math.round(minMb.value * 1024 * 1024), level: loadLevel(), ...(scope.value ? { under: [scope.value] } : {}) }
    const res = await diskMapDupes(req)
    if (!res.started) {
      running.value = false
      error.value = dupeError(res.error)
      return false
    }
    return true
  }

  async function cancel(): Promise<void> {
    if (!running.value) return
    await diskMapDupesCancel()
  }

  async function togglePause(): Promise<void> {
    if (!running.value) return
    const ok = paused.value ? await diskMapResume() : await diskMapPause()
    if (ok) paused.value = !paused.value
  }

  function setKeep(group: DiskDupeGroup, path: string): void {
    const next = keepOne(ticked.value, group, path)
    if (next) ticked.value = next
  }

  function toggleFile(group: DiskDupeGroup, path: string): boolean {
    const next = toggleTick(ticked.value, group, path)
    if (!next) return false
    ticked.value = next
    return true
  }

  function resetTicks(): void {
    ticked.value = defaultTicked(groups.value)
  }

  function clearTicks(): void {
    ticked.value = new Set()
  }

  function toggleOpen(key: string): void {
    const next = new Set(expanded.value)
    if (next.has(key)) next.delete(key)
    else next.add(key)
    expanded.value = next
  }

  function patchFilters(patch: Partial<DupeFilters>): void {
    filters.value = { ...filters.value, ...patch }
  }

  function reveal(path: string): Promise<void> {
    return diskMapReveal(path)
  }

  function dismissReport(): void {
    report.value = null
  }

  async function remove(permanent: boolean): Promise<void> {
    if (deleting.value) return
    const paths = tickedPaths(shown.value, ticked.value).slice(0, DELETE_BATCH)
    if (!paths.length) return
    deleting.value = true
    report.value = null
    const res = await diskMapDelete(paths, permanent)
    deleting.value = false
    if (!res) {
      report.value = { freed: 0, done: 0, failed: paths.map((p) => ({ path: p, reason: 'unavailable' })), permanent }
      return
    }
    if (res.cancelled) return
    const gone = new Set(res.done.map((d) => d.path))
    const next = new Set(ticked.value)
    for (const g of gone) next.delete(g)
    groups.value = removePaths(groups.value, gone)
    ticked.value = next
    triggerRef(groups)
    report.value = { freed: res.freed, done: res.done.length, failed: res.failed, permanent: res.permanent }
    const dm = useDiskMapStore()
    void dm.loadHistory()
    void dm.reload()
  }

  function open(): void {
    active.value = true
    init()
  }

  return {
    supported,
    active,
    running,
    paused,
    progress,
    result,
    groups,
    ticked,
    expanded,
    scope,
    minMb,
    filters,
    error,
    detail,
    deleting,
    report,
    shown,
    plan,
    wasted,
    shownWasted,
    hasResult,
    init,
    dispose,
    open,
    start,
    cancel,
    togglePause,
    setKeep,
    toggleFile,
    resetTicks,
    clearTicks,
    toggleOpen,
    patchFilters,
    reveal,
    remove,
    dismissReport,
    key: groupKey,
  }
})
