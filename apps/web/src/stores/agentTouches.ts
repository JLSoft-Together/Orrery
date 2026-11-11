import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'
import { useActiveInterval } from '../composables/usePageActive'
import { buildTouchIndex, type TouchIndex, type TouchTerminal } from '../lib/agentTouches'
import { displayTitle } from '../lib/title'
import { useTerminalsStore } from './terminals'
import { useToolSpansStore } from './toolSpans'

const TICK_MS = 15_000

interface Edge {
  turnStart?: number
  idleSince?: number
  working: boolean
}

export const useAgentTouchesStore = defineStore('agentTouches', () => {
  const terminals = useTerminalsStore()
  const spans = useToolSpansStore()
  const now = ref(Date.now())
  const edges = ref(new Map<string, Edge>())

  const signature = computed(() => terminals.list.map((t) => `${t.id}:${t.status === 'working' ? 1 : 0}`).join('|'))
  watch(
    signature,
    () => {
      const next = new Map<string, Edge>()
      const at = Date.now()
      for (const t of terminals.list) {
        const prev = edges.value.get(t.id)
        if (t.status === 'working') next.set(t.id, { working: true, turnStart: !prev ? undefined : prev.working ? prev.turnStart : at })
        else if (prev?.working) next.set(t.id, { working: false, turnStart: prev.turnStart, idleSince: at })
        else if (prev) next.set(t.id, prev)
      }
      edges.value = next
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

  return { index }
})
