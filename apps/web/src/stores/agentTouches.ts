import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'
import { useActiveInterval } from '../composables/usePageActive'
import { buildTouchIndex, type TouchIndex, type TouchTerminal } from '../lib/agentTouches'
import { addIgnored, clashesByTerminal, clashList, newClashes, pairKeys, parseIgnored, pruneIgnored, removeIgnored, type Clash, type IgnoredFile } from '../lib/clash'
import { displayTitle } from '../lib/title'
import { edgeSignature, stepEdges, type TurnEdge } from '../lib/turnEdges'
import { useSettingsStore } from './settings'
import { useTerminalsStore } from './terminals'
import { useToolSpansStore } from './toolSpans'

const TICK_MS = 15_000
const IGNORED_KEY = 'ccm.clash.ignored'
const MAX_REPORTED = 2000

function readIgnored(): IgnoredFile[] {
  try {
    return parseIgnored(localStorage.getItem(IGNORED_KEY))
  } catch {
    return []
  }
}

function saveIgnored(list: readonly IgnoredFile[]): void {
  try {
    if (list.length) localStorage.setItem(IGNORED_KEY, JSON.stringify(list))
    else localStorage.removeItem(IGNORED_KEY)
  } catch {
    return
  }
}

export const useAgentTouchesStore = defineStore('agentTouches', () => {
  const terminals = useTerminalsStore()
  const settings = useSettingsStore()
  const spans = useToolSpansStore()
  const now = ref(Date.now())
  const edges = ref(new Map<string, TurnEdge>())

  const signature = computed(() => edgeSignature(terminals.list))
  watch(
    signature,
    () => {
      const at = Date.now()
      edges.value = stepEdges(edges.value, terminals.list, at)
      now.value = at
    },
    { immediate: true },
  )

  useActiveInterval(() => (now.value = Date.now()), TICK_MS)

  const index = computed<TouchIndex>(() => {
    void spans.version
    const terms = new Map<string, TouchTerminal>()
    for (const t of terminals.list) {
      const e = edges.value.get(t.id)
      terms.set(t.id, { provider: t.provider, name: displayTitle(t), working: e?.working ?? false, turnStart: e?.turnStart, idleSince: e?.idleSince })
    }
    return buildTouchIndex(spans.spansFor(), terms, now.value)
  })

  const ignored = ref<IgnoredFile[]>(readIgnored())
  const ignoredKeys = computed(() => new Set(ignored.value.map((x) => x.key)))
  const allClashes = computed<Clash[]>(() => (settings.clashWarn ? clashList(index.value) : []))
  const clashes = computed<Clash[]>(() => allClashes.value.filter((c) => !ignoredKeys.value.has(c.fileKey)))
  const clashesOf = computed(() => clashesByTerminal(clashes.value))
  const reported = new Set<string>()

  function setIgnored(list: IgnoredFile[]): void {
    ignored.value = list
    saveIgnored(list)
  }

  function ignoreFile(fileKey: string): void {
    const ids = allClashes.value.find((c) => c.fileKey === fileKey)?.peers.map((p) => p.terminalId) ?? []
    setIgnored(addIgnored(ignored.value, { key: fileKey, ids }))
  }

  function unignoreFile(fileKey: string): void {
    if (ignoredKeys.value.has(fileKey)) setIgnored(removeIgnored(ignored.value, fileKey))
  }

  const turnKey = (id: string): string => String(edges.value.get(id)?.turnStart ?? '')

  function takeNew(): Clash[] {
    const fresh = newClashes(reported, clashes.value, turnKey)
    const live = new Set<string>()
    for (const c of allClashes.value) for (const k of pairKeys(c, turnKey)) live.add(k)
    for (const k of reported) if (!live.has(k)) reported.delete(k)
    for (const c of fresh) for (const k of pairKeys(c, turnKey)) if (reported.size < MAX_REPORTED) reported.add(k)
    return fresh
  }

  watch(
    () => terminals.live.map((t) => t.id).join('|'),
    () => {
      if (!ignored.value.length || !terminals.list.length) return
      const alive = new Set(terminals.live.map((t) => t.id))
      const next = pruneIgnored(ignored.value, (id) => alive.has(id))
      if (next.length !== ignored.value.length) setIgnored(next)
    },
  )

  return { edges, index, clashes, clashesOf, ignored, ignoreFile, unignoreFile, takeNew }
})
