import { defineStore } from 'pinia'
import { computed, shallowReactive } from 'vue'
import { normalizeAgentStatus, type Agent, type AgentPatch } from '@ccm/shared'
import { applyEntityOps, type EntityOp } from '../lib/patch'

export const useAgentsStore = defineStore('agents', () => {
  const byId = shallowReactive<Record<string, Agent>>({})

  const list = computed(() => Object.values(byId))

  const byTerminal = computed(() => {
    const map: Record<string, Agent[]> = {}
    for (const a of list.value) (map[a.terminalId] ??= []).push(a)
    for (const agents of Object.values(map)) {
      agents.sort(
        (a, b) =>
          Number(a.role !== 'main') - Number(b.role !== 'main') ||
          (a.startedAt ?? '').localeCompare(b.startedAt ?? '') ||
          a.id.localeCompare(b.id),
      )
    }
    return map
  })

  const subagents = computed(() => list.value.filter((a) => a.role === 'subagent'))

  function sanitize(a: Agent): Agent {
    return { ...a, status: normalizeAgentStatus(a.status), role: a.role === 'main' ? 'main' : 'subagent' }
  }

  function replaceAll(items: Agent[]): void {
    for (const id of Object.keys(byId)) delete byId[id]
    for (const a of items) byId[a.id] = Object.freeze(sanitize(a))
  }

  function applyMany(ops: readonly EntityOp<Agent, AgentPatch>[]): void {
    applyEntityOps(byId, ops, sanitize)
  }

  function upsert(a: Agent): void {
    applyMany([{ kind: 'upsert', value: a }])
  }

  function patch(p: AgentPatch): void {
    applyMany([{ kind: 'patch', patch: p }])
  }

  function remove(id: string): void {
    applyMany([{ kind: 'remove', id }])
  }

  function mainOf(terminalId: string): Agent | undefined {
    return byTerminal.value[terminalId]?.find((a) => a.role === 'main')
  }

  function lineage(id: string): Set<string> {
    const out = new Set<string>([id])
    let cur = byId[id]
    while (cur?.parentId && !out.has(cur.parentId)) {
      out.add(cur.parentId)
      cur = byId[cur.parentId]
    }
    const queue = [id]
    while (queue.length) {
      const parent = queue.shift()!
      for (const a of list.value) {
        if (a.parentId === parent && !out.has(a.id)) {
          out.add(a.id)
          queue.push(a.id)
        }
      }
    }
    return out
  }

  return { byId, list, byTerminal, subagents, replaceAll, applyMany, upsert, patch, remove, mainOf, lineage }
})
