import { defineStore } from 'pinia'
import { computed, shallowReactive } from 'vue'
import { normalizeTerminalStatus, type TerminalPatch, type TerminalSession } from '@ccm/shared'
import { applyEntityOps, type EntityOp } from '../lib/patch'

export const useTerminalsStore = defineStore('terminals', () => {
  const byId = shallowReactive<Record<string, TerminalSession>>({})

  const list = computed(() =>
    Object.values(byId).sort(
      (a, b) =>
        Number(a.status === 'stale') - Number(b.status === 'stale') ||
        (a.startedAt ?? '').localeCompare(b.startedAt ?? '') ||
        a.id.localeCompare(b.id),
    ),
  )

  const live = computed(() => list.value.filter((t) => t.status !== 'stale'))

  function sanitize(t: TerminalSession): TerminalSession {
    return { ...t, status: normalizeTerminalStatus(t.status), title: t.title || t.id }
  }

  function replaceAll(items: TerminalSession[]): void {
    for (const id of Object.keys(byId)) delete byId[id]
    for (const t of items) byId[t.id] = Object.freeze(sanitize(t))
  }

  function applyMany(ops: readonly EntityOp<TerminalSession, TerminalPatch>[]): void {
    applyEntityOps(byId, ops, sanitize)
  }

  function upsert(t: TerminalSession): void {
    applyMany([{ kind: 'upsert', value: t }])
  }

  function patch(p: TerminalPatch): void {
    applyMany([{ kind: 'patch', patch: p }])
  }

  function remove(id: string): void {
    applyMany([{ kind: 'remove', id }])
  }

  return { byId, list, live, replaceAll, applyMany, upsert, patch, remove }
})
