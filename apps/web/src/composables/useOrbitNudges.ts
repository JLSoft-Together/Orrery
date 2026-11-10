import { shallowRef, watch } from 'vue'
import { useAgentsStore } from '../stores/agents'

export function useOrbitNudges() {
  const agents = useAgentsStore()
  const nudges = shallowRef<Readonly<Record<string, number>>>({})

  watch(
    () => agents.list.map((a) => `${a.id}\u0000${a.currentToolStartedAt ?? ''}`),
    (rows, before) => {
      if (!before) return
      const prev = new Map(before.map((r) => r.split('\u0000') as [string, string]))
      const next = { ...nudges.value }
      let changed = false
      for (const r of rows) {
        const [id, at] = r.split('\u0000') as [string, string]
        const was = prev.get(id)
        if (!at || was === undefined || was === at) continue
        next[id] = (next[id] ?? 0) + 1
        changed = true
      }
      if (changed) nudges.value = next
    },
  )

  return nudges
}
