import { defineStore } from 'pinia'
import { computed, ref, shallowRef } from 'vue'
import type { DiskEntry } from '@ccm/shared'
import {
  diskScheduleGet,
  diskScheduleSet,
  diskSnapCompare,
  diskSnapList,
  findRow,
  hasDiskSchedule,
  hasDiskSnap,
  indexRows,
  onDiskSchedule,
  onDiskSnapNew,
  sortByDelta,
  type DiffIndex,
  type DiskCompare,
  type DiskDiffRow,
  type DiskSchedule,
  type DiskSchedulePatch,
  type DiskSnapshotMeta,
} from '../lib/diskSnap'

const EMPTY: DiffIndex = { exact: new Map(), folded: new Map() }

export const useDiskSnapStore = defineStore('diskSnap', () => {
  const snapshots = shallowRef<DiskSnapshotMeta[]>([])
  const baseId = ref('')
  const compare = shallowRef<DiskCompare | null>(null)
  const index = shallowRef<DiffIndex>(EMPTY)
  const top = shallowRef<DiskDiffRow[]>([])
  const loading = ref(false)
  const failed = ref(false)
  const sortDelta = ref(false)
  const showTop = ref(false)
  const schedule = shallowRef<DiskSchedule | null>(null)
  const supported = hasDiskSnap
  let started = false
  let offs: (() => void)[] = []
  let seq = 0
  let lastPath = ''

  const active = computed(() => supported && !!baseId.value && !!compare.value)
  const base = computed(() => compare.value?.base ?? snapshots.value.find((s) => s.id === baseId.value) ?? null)
  const covered = computed(() => compare.value?.covered === true)

  async function loadList(): Promise<void> {
    snapshots.value = await diskSnapList()
    if (baseId.value && !snapshots.value.some((s) => s.id === baseId.value)) clear()
  }

  async function init(): Promise<void> {
    if (!started) {
      started = true
      offs.push(
        onDiskSnapNew(() => {
          void loadList()
          if (baseId.value) void load(lastPath)
        }),
        onDiskSchedule((s) => (schedule.value = s)),
      )
    }
    await Promise.all([hasDiskSnap ? loadList() : Promise.resolve(), initSchedule()])
  }

  async function initSchedule(): Promise<void> {
    if (!hasDiskSchedule) return
    const s = await diskScheduleGet()
    if (s) schedule.value = s
  }

  function dispose(): void {
    for (const off of offs) off()
    offs = []
    started = false
    seq++
  }

  function clear(): void {
    seq++
    baseId.value = ''
    compare.value = null
    index.value = EMPTY
    top.value = []
    loading.value = false
    failed.value = false
    showTop.value = false
    sortDelta.value = false
  }

  async function load(path: string): Promise<void> {
    if (!baseId.value) return
    const mine = ++seq
    lastPath = path
    loading.value = true
    const res = await diskSnapCompare(baseId.value, path)
    if (mine !== seq) return
    loading.value = false
    failed.value = res === null
    if (!res) {
      compare.value = null
      index.value = EMPTY
      top.value = []
      return
    }
    compare.value = res
    index.value = indexRows(res.rows)
    top.value = res.top
  }

  async function select(id: string, path: string): Promise<void> {
    if (!id) return clear()
    baseId.value = id
    await load(path)
  }

  async function sync(path: string): Promise<void> {
    if (!baseId.value || (path === lastPath && compare.value)) return
    await load(path)
  }

  function rowOf(path: string): DiskDiffRow | undefined {
    return active.value ? findRow(index.value, path) : undefined
  }

  function order(items: readonly DiskEntry[]): readonly DiskEntry[] {
    return active.value && sortDelta.value ? sortByDelta(items, index.value) : items
  }

  function toggleSort(): void {
    sortDelta.value = !sortDelta.value
  }

  async function setSchedule(patch: DiskSchedulePatch): Promise<void> {
    const s = await diskScheduleSet(patch)
    if (s) schedule.value = s
  }

  return { snapshots, baseId, compare, top, loading, failed, sortDelta, showTop, schedule, supported, active, base, covered, init, dispose, loadList, select, sync, clear, rowOf, order, toggleSort, setSchedule }
})
