import { defineStore } from 'pinia'
import { computed, shallowRef, watch } from 'vue'
import { displayTitle } from '../../../core/lib/title'
import { addPaths, editPaths, mergeTurn, parseTurns, serializeTurns, TURN_EDITS_KEY, turnStartOf, type TurnEdits } from '../lib/turnEdits'
import { useAgentTouchesStore } from './agentTouches'
import { useConnectionStore } from '../../../app/stores/connection'
import { useTerminalsStore } from '../../../core/stores/terminals'
import { useToolSpansStore } from '../../../core/stores/toolSpans'

const SAVE_MS = 400

function readSaved(): ReturnType<typeof parseTurns> {
  try {
    return parseTurns(sessionStorage.getItem(TURN_EDITS_KEY))
  } catch {
    return { current: [], last: [] }
  }
}

export const useTurnEditsStore = defineStore('turnEdits', () => {
  const terminals = useTerminalsStore()
  const touches = useAgentTouchesStore()
  const spans = useToolSpansStore()
  const connection = useConnectionStore()
  const saved = readSaved()
  const current = shallowRef(new Map(saved.current.map((t) => [t.terminalId, t])))
  const last = shallowRef(new Map(saved.last.map((t) => [t.terminalId, t])))

  let saveTimer = 0
  function save(): void {
    if (saveTimer) return
    saveTimer = window.setTimeout(() => {
      saveTimer = 0
      try {
        if (current.value.size || last.value.size) sessionStorage.setItem(TURN_EDITS_KEY, serializeTurns(current.value.values(), last.value.values()))
        else sessionStorage.removeItem(TURN_EDITS_KEY)
      } catch {
        return
      }
    }, SAVE_MS)
  }

  function commit(nextCurrent: Map<string, TurnEdits>, nextLast: Map<string, TurnEdits>): void {
    if (nextCurrent !== current.value) current.value = nextCurrent
    if (nextLast !== last.value) last.value = nextLast
    save()
  }

  const nameOf = (id: string, fallback = ''): string => {
    const x = terminals.byId[id]
    return x ? displayTitle(x) || fallback : fallback
  }

  function fresh(turn: TurnEdits, to?: number): TurnEdits {
    return addPaths(turn, editPaths(spans.spansFor(turn.terminalId), turn.turnStart, to))
  }

  watch(
    () => touches.edges,
    (next, prev) => {
      const at = Date.now()
      const cur = new Map(current.value)
      const done = new Map(last.value)
      let changed = false
      for (const [id, edge] of next) {
        const before = prev?.get(id)
        const open = cur.get(id)
        if (edge.working) {
          if (open && (!before || before.working)) continue
          cur.set(id, fresh({ terminalId: id, name: nameOf(id), turnStart: turnStartOf(edge.turnStart, at), paths: new Set() }))
          changed = true
        } else if (open) {
          cur.delete(id)
          changed = true
          const end = edge.idleSince ?? at
          const closed = fresh({ ...open, name: nameOf(id, open.name), turnEnd: end }, end)
          if (closed.paths.size) done.set(id, mergeTurn(done.get(id), closed))
        }
      }
      if (changed) commit(cur, done)
    },
    { immediate: true },
  )

  watch(
    () => spans.version,
    () => {
      let cur = current.value
      let done = last.value
      for (const [id, turn] of cur) {
        const next = fresh(turn)
        if (next !== turn) (cur === current.value ? (cur = new Map(cur)) : cur).set(id, next)
      }
      for (const [id, turn] of done) {
        const next = fresh(turn, turn.turnEnd)
        if (next !== turn) (done === last.value ? (done = new Map(done)) : done).set(id, next)
      }
      if (cur !== current.value || done !== last.value) commit(cur, done)
    },
  )

  watch(
    () => [connection.hasData, terminals.list.map((t) => `${t.id}:${t.status === 'stale' ? 0 : 1}`).join('|')] as const,
    ([ready]) => {
      if (!ready) return
      const alive = (id: string): boolean => {
        const x = terminals.byId[id]
        return !!x && x.status !== 'stale'
      }
      const cur = new Map([...current.value].filter(([id]) => alive(id)))
      const done = new Map([...last.value].filter(([id]) => alive(id)))
      if (cur.size !== current.value.size || done.size !== last.value.size) commit(cur, done)
    },
    { immediate: true },
  )

  const turns = computed<TurnEdits[]>(() => {
    const out = new Map<string, TurnEdits>()
    for (const t of last.value.values()) out.set(t.terminalId, t)
    for (const t of current.value.values()) out.set(t.terminalId, mergeTurn(out.get(t.terminalId), t))
    return [...out.values()].filter((t) => t.paths.size).map((t) => ({ ...t, name: nameOf(t.terminalId, t.name) || t.terminalId }))
  })

  const countOf = (id: string): number => turns.value.find((t) => t.terminalId === id)?.paths.size ?? 0

  function markSeen(id: string): void {
    if (!last.value.has(id)) return
    const done = new Map(last.value)
    done.delete(id)
    commit(current.value, done)
  }

  return { current, last, turns, countOf, markSeen }
})
