import { computed, ref, shallowRef } from 'vue'
import { defineStore } from 'pinia'
import {
  hasOrganize,
  junkPreselect,
  onOrganizeChanged,
  organizeAddSource,
  organizeDelete,
  organizeJunk,
  organizePlan,
  organizeRemoveSource,
  organizeRun,
  organizeSettings,
  organizeState,
  organizeUndo,
  ruleValid,
  type OrganizeJunkItem,
  type OrganizeKind,
  type OrganizeMode,
  type OrganizePlan,
  type OrganizeRule,
  type OrganizeRunResult,
  type OrganizeState,
  type OrganizeUndoResult,
} from '../lib/organize'

export const useOrganizeStore = defineStore('organize', () => {
  const supported = hasOrganize
  const active = ref(false)
  const state = shallowRef<OrganizeState | null>(null)
  const plan = shallowRef<OrganizePlan | null>(null)
  const off = ref<Set<string>>(new Set())
  const planning = ref(false)
  const running = ref(false)
  const result = shallowRef<OrganizeRunResult | null>(null)
  const undoResult = shallowRef<(OrganizeUndoResult & { run: string }) | null>(null)
  const undoing = ref<string | null>(null)
  const sourceError = ref<string | null>(null)
  const junk = shallowRef<OrganizeJunkItem[]>([])
  const junkScanned = ref(false)
  const junkIndexed = ref(true)
  const junkTruncated = ref(false)
  const junkScanning = ref(false)
  const junkPicked = ref<Set<string>>(new Set())
  let unsub: (() => void) | null = null

  const settings = computed(() => state.value?.settings ?? null)
  const sources = computed(() => state.value?.sources ?? [])
  const runs = computed(() => state.value?.runs ?? [])
  const chosen = computed(() => (plan.value?.items ?? []).filter((i) => !off.value.has(i.id)))
  const chosenBytes = computed(() => chosen.value.reduce((s, i) => s + i.size, 0))
  const junkChosen = computed(() => junk.value.filter((i) => junkPicked.value.has(i.path)))

  function apply(next: OrganizeState | null): void {
    if (next) state.value = next
  }

  async function load(): Promise<void> {
    apply(await organizeState())
  }

  async function init(): Promise<void> {
    if (!supported) return
    unsub ??= onOrganizeChanged(() => void load())
    await load()
  }

  function dispose(): void {
    unsub?.()
    unsub = null
  }

  function resetPlan(): void {
    plan.value = null
    off.value = new Set()
  }

  async function patch(p: Parameters<typeof organizeSettings>[0]): Promise<void> {
    apply(await organizeSettings(p))
    if (p.source !== undefined || p.mode !== undefined || p.kinds !== undefined || p.rules !== undefined) resetPlan()
  }

  const setSource = (source: string): Promise<void> => patch({ source })
  const setMode = (mode: OrganizeMode): Promise<void> => patch({ mode })
  const setAuto = (auto: boolean): Promise<void> => patch({ auto })

  function toggleKind(kind: OrganizeKind): Promise<void> {
    const cur = new Set(settings.value?.kinds ?? [])
    if (cur.has(kind)) cur.delete(kind)
    else cur.add(kind)
    return patch({ kinds: [...cur] })
  }

  function saveRules(rules: OrganizeRule[]): Promise<void> {
    return patch({ rules: rules.filter(ruleValid).map((r) => ({ kind: r.kind, value: String(r.value).trim(), folder: String(r.folder).trim() })) })
  }

  async function addSource(kind: 'downloads' | 'desktop' | 'pick'): Promise<void> {
    sourceError.value = null
    const r = await organizeAddSource(kind)
    if (!r) return
    apply(r.state)
    if (r.reason) sourceError.value = r.reason
    else resetPlan()
  }

  async function removeSource(p: string): Promise<void> {
    apply(await organizeRemoveSource(p))
    resetPlan()
  }

  async function preview(): Promise<void> {
    const src = settings.value?.source
    if (!src || planning.value) return
    planning.value = true
    result.value = null
    try {
      plan.value = await organizePlan(src)
      off.value = new Set()
    } finally {
      planning.value = false
    }
  }

  function toggleItem(id: string): void {
    const next = new Set(off.value)
    if (next.has(id)) next.delete(id)
    else next.add(id)
    off.value = next
  }

  function setAll(on: boolean): void {
    off.value = on ? new Set() : new Set((plan.value?.items ?? []).map((i) => i.id))
  }

  async function run(): Promise<OrganizeRunResult | null> {
    const p = plan.value
    if (!p || running.value || !chosen.value.length) return null
    running.value = true
    try {
      const r = await organizeRun(p.id, [...off.value])
      result.value = r
      resetPlan()
      await load()
      return r
    } finally {
      running.value = false
    }
  }

  async function undo(id: string): Promise<OrganizeUndoResult | null> {
    if (undoing.value) return null
    undoing.value = id
    try {
      const r = await organizeUndo(id)
      undoResult.value = r ? { ...r, run: id } : null
      await load()
      return r
    } finally {
      undoing.value = null
    }
  }

  async function scanJunk(): Promise<void> {
    if (junkScanning.value) return
    junkScanning.value = true
    try {
      const r = await organizeJunk()
      junk.value = r.items
      junkIndexed.value = r.indexed
      junkTruncated.value = !!r.truncated
      junkPicked.value = junkPreselect(r.items)
      junkScanned.value = true
    } finally {
      junkScanning.value = false
    }
  }

  function toggleJunk(p: string): void {
    const next = new Set(junkPicked.value)
    if (next.has(p)) next.delete(p)
    else next.add(p)
    junkPicked.value = next
  }

  async function removeJunk(): Promise<number> {
    const paths = junkChosen.value.map((i) => i.path)
    if (!paths.length) return 0
    const r = await organizeDelete(paths)
    if (!r || r.cancelled) return 0
    const gone = new Set(r.done.map((d) => d.path))
    junk.value = junk.value.filter((i) => !gone.has(i.path))
    junkPicked.value = new Set([...junkPicked.value].filter((p) => !gone.has(p)))
    return gone.size
  }

  function dismiss(): void {
    result.value = null
    undoResult.value = null
    sourceError.value = null
  }

  return {
    supported,
    active,
    state,
    settings,
    sources,
    runs,
    plan,
    off,
    chosen,
    chosenBytes,
    planning,
    running,
    result,
    undoResult,
    undoing,
    sourceError,
    junk,
    junkScanned,
    junkIndexed,
    junkTruncated,
    junkScanning,
    junkPicked,
    junkChosen,
    init,
    dispose,
    load,
    setSource,
    setMode,
    setAuto,
    toggleKind,
    saveRules,
    addSource,
    removeSource,
    preview,
    toggleItem,
    setAll,
    run,
    undo,
    scanJunk,
    toggleJunk,
    removeJunk,
    dismiss,
  }
})
