import { defineStore } from 'pinia'
import { shallowRef, watch } from 'vue'
import type { WorkflowStatus } from '@ccm/shared'
import { hasAgentStatus, onAgentStatus, watchAgentStatus } from '../lib/agentStatus'
import { isTerm, presetOf, useWorkspaceStore, workCwd } from './workspace'

/** Workflow status of agent panes (docs §11ap): desktop main watches `<pane folder>/.orrery/status.json`; the page only reports folders. */
export const useAgentStatusStore = defineStore('agentStatus', () => {
  const workspace = useWorkspaceStore()
  const byKey = shallowRef<Record<string, WorkflowStatus>>({})
  const asked = new Map<string, string>()
  let started = false

  const statusOf = (paneKey: string): WorkflowStatus | undefined => byKey.value[paneKey]

  function set(key: string, status: WorkflowStatus | null): void {
    if (!status && !(key in byKey.value)) return
    const next = { ...byKey.value }
    if (status) next[key] = status
    else delete next[key]
    byKey.value = next
  }

  function wanted(): Map<string, string> {
    const out = new Map<string, string>()
    for (const p of workspace.panes) {
      const dir = isTerm(p) && presetOf(p) !== 'shell' && p.state === 'running' ? workCwd(p) : undefined
      if (dir) out.set(p.key, dir)
    }
    return out
  }

  function sync(): void {
    const now = wanted()
    for (const key of [...asked.keys()]) {
      if (now.has(key)) continue
      asked.delete(key)
      set(key, null)
      void watchAgentStatus(key, null)
    }
    for (const [key, dir] of now) {
      if (asked.get(key) === dir) continue
      asked.set(key, dir)
      void watchAgentStatus(key, dir)
    }
  }

  function start(): void {
    if (started || !hasAgentStatus) return
    started = true
    onAgentStatus((e) => {
      if (asked.has(e.key)) set(e.key, e.status)
    })
    watch(
      () => [...wanted()].map(([k, d]) => `${k}\n${d}`).join('\n\n'),
      sync,
      { immediate: true },
    )
  }

  return { byKey, statusOf, start }
})
