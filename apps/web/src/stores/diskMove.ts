import { computed, ref, shallowRef } from 'vue'
import { defineStore } from 'pinia'
import {
  diskMoveBack,
  diskMoveCancel,
  diskMoveFromClean,
  diskMoveList,
  diskMovePick,
  diskMovePlan,
  diskMoveRecover,
  diskMoveRun,
  diskMoveState,
  diskMoveVolumes,
  hasDiskMove,
  onDiskMoveProgress,
  type DiskMoveRecord,
  type MoveCandidate,
  type MovePending,
  type MovePlan,
  type MoveProgress,
  type MoveResult,
  type MoveVolume,
} from '../lib/diskMove'

export interface MoveNotice {
  ok: boolean
  kind: 'move' | 'back' | 'restore' | 'finish' | 'plan'
  error?: string
  holder?: string
  leftover?: string
  rolledBack?: boolean
}

export const useDiskMoveStore = defineStore('diskMove', () => {
  const supported = hasDiskMove
  const candidates = shallowRef<MoveCandidate[]>([])
  const moves = shallowRef<DiskMoveRecord[]>([])
  const pending = shallowRef<MovePending[]>([])
  const chosen = ref<string | null>(null)
  const volumes = shallowRef<MoveVolume[]>([])
  const volume = ref<string | null>(null)
  const plan = shallowRef<MovePlan | null>(null)
  const planning = ref(false)
  const running = ref(false)
  const progress = shallowRef<MoveProgress | null>(null)
  const notice = shallowRef<MoveNotice | null>(null)
  const loading = ref(false)
  let off: (() => void) | null = null

  const current = computed(() => candidates.value.find((c) => c.id === chosen.value) ?? null)
  const busy = computed(() => planning.value || running.value)

  async function refresh(): Promise<void> {
    if (!supported) return
    loading.value = true
    try {
      const [list, st] = await Promise.all([diskMoveList(), diskMoveState()])
      candidates.value = list
      moves.value = st.moves
      pending.value = st.pending
    } finally {
      loading.value = false
    }
  }

  async function init(): Promise<void> {
    if (!supported) return
    if (!off) off = onDiskMoveProgress((p) => (progress.value = p))
    await refresh()
  }

  function dispose(): void {
    off?.()
    off = null
  }

  function addCandidate(c: MoveCandidate): void {
    candidates.value = [...candidates.value.filter((x) => x.id !== c.id), c]
  }

  async function choose(id: string | null): Promise<void> {
    chosen.value = id
    plan.value = null
    volume.value = null
    volumes.value = []
    notice.value = null
    if (!id) return
    volumes.value = await diskMoveVolumes(id)
    const first = volumes.value.find((v) => v.ok)
    if (first) volume.value = first.mount
  }

  async function pick(): Promise<void> {
    const c = await diskMovePick()
    if (!c) return
    addCandidate(c)
    if (!c.block) await choose(c.id)
  }

  async function fromClean(path: string, open = false): Promise<boolean> {
    const c = await diskMoveFromClean(path, open)
    if (!c) return false
    addCandidate(c)
    if (c.block) notice.value = { ok: false, kind: 'plan', error: c.block }
    else await choose(c.id)
    return true
  }

  async function makePlan(): Promise<void> {
    if (!chosen.value || !volume.value || busy.value) return
    planning.value = true
    progress.value = null
    notice.value = null
    try {
      const res = await diskMovePlan(chosen.value, volume.value)
      plan.value = res.plan ?? null
      if (!res.plan) notice.value = { ok: false, kind: 'plan', error: res.error, ...(res.holder ? { holder: res.holder } : {}) }
    } finally {
      planning.value = false
    }
  }

  function noticeOf(kind: MoveNotice['kind'], r: MoveResult): MoveNotice | null {
    if (r.cancelled && !r.rolledBack) return null
    return { ok: r.ok, kind, ...(r.error ? { error: r.error } : {}), ...(r.holder ? { holder: r.holder } : {}), ...(r.leftover ? { leftover: r.leftover } : {}), ...(r.rolledBack ? { rolledBack: true } : {}) }
  }

  async function run(): Promise<MoveResult | null> {
    if (!plan.value || busy.value) return null
    running.value = true
    progress.value = null
    try {
      const r = await diskMoveRun(plan.value.id)
      notice.value = noticeOf('move', r)
      if (r.ok || !r.cancelled) {
        plan.value = null
        chosen.value = null
      }
      await refresh()
      return r
    } finally {
      running.value = false
      progress.value = null
    }
  }

  async function back(id: string): Promise<MoveResult | null> {
    if (busy.value) return null
    running.value = true
    progress.value = null
    try {
      const r = await diskMoveBack(id)
      notice.value = noticeOf('back', r)
      await refresh()
      return r
    } finally {
      running.value = false
      progress.value = null
    }
  }

  async function recover(id: string, action: 'restore' | 'finish'): Promise<void> {
    if (busy.value) return
    running.value = true
    try {
      notice.value = noticeOf(action, await diskMoveRecover(id, action))
      await refresh()
    } finally {
      running.value = false
    }
  }

  async function cancel(): Promise<void> {
    await diskMoveCancel()
  }

  function dismiss(): void {
    notice.value = null
  }

  return { supported, candidates, moves, pending, chosen, current, volumes, volume, plan, planning, running, busy, progress, notice, loading, init, dispose, refresh, choose, pick, fromClean, makePlan, run, back, recover, cancel, dismiss }
})
