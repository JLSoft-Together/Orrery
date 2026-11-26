import { defineStore } from 'pinia'
import { ref, watch } from 'vue'
import type { RunSession } from '@ccm/shared'
import { dirKey } from '../../../core/stores/favorites'
import { buildDeps, type BuildOutcome } from '../lib/multiRunDeps'
import { pickDevices, runMany, variantOf, type MultiOutcome, type MultiStage, type VariantId } from '../lib/multiRun'
import { useRunStore } from './run'

export const SESSION_WAIT_MS = 5000
const ENDED: readonly RunSession['state'][] = ['failed', 'stopped', 'cancelled']

export interface MultiRunState {
  variant: VariantId
  running: boolean
  cancelling: boolean
  stages: Record<string, { stage: MultiStage; error?: string }>
  outcome: MultiOutcome | null
}

interface Ctl {
  cancelled: boolean
  session: string | null
}

export const useMultiRunStore = defineStore('multiRun', () => {
  const run = useRunStore()
  const states = ref<Record<string, MultiRunState>>({})
  const ctls = new Map<string, Ctl>()

  const stateFor = (root: string): MultiRunState | null => states.value[dirKey(root)] ?? null
  const isRunning = (root: string): boolean => !!stateFor(root)?.running

  function settle(sessionId: string): Promise<BuildOutcome> {
    return new Promise((resolve) => {
      let done = false
      let off: (() => void) | null = null
      let timer: ReturnType<typeof setTimeout> | null = null
      const finish = (o: BuildOutcome): void => {
        if (done) return
        done = true
        off?.()
        if (timer) clearTimeout(timer)
        resolve(o)
      }
      const check = (): void => {
        const s = run.sessions[sessionId]
        if (!s) return
        if (s.state === 'running') finish({ ok: true, failure: null })
        else if (ENDED.includes(s.state)) finish({ ok: false, failure: s.failure ?? { code: s.state } })
      }
      off = watch(() => run.sessions[sessionId]?.state, check, { immediate: true })
      if (done) off()
      else if (!run.sessions[sessionId]) timer = setTimeout(() => finish({ ok: false, failure: { code: 'session-missing' } }), SESSION_WAIT_MS)
    })
  }

  async function startMany(root: string, configId: string, deviceIds: readonly string[], variantId: VariantId): Promise<MultiOutcome | null> {
    const key = dirKey(root)
    const cfg = run.configsFor(root).find((c) => c.id === configId)
    const ids = pickDevices(deviceIds)
    if (states.value[key]?.running || !cfg || cfg.type !== 'android' || !ids.length) return null
    const ctl: Ctl = { cancelled: false, session: null }
    ctls.set(key, ctl)
    states.value[key] = { variant: variantId, running: true, cancelling: false, stages: Object.fromEntries(ids.map((id) => [id, { stage: 'waiting' as MultiStage }])), outcome: null }
    const deps = buildDeps(cfg, async (id) => {
      const res = await run.startConfig(root, configId, id)
      if (!res.ok) return { ok: false, failure: res.error }
      if (!res.sessionId) return { ok: false, failure: { code: 'session-missing' } }
      ctl.session = res.sessionId
      if (ctl.cancelled) void run.stop(res.sessionId, root)
      return settle(res.sessionId)
    })
    try {
      const outcome = await runMany({
        devices: ids,
        variant: variantOf(variantId),
        deps,
        isCancelled: () => ctl.cancelled,
        onStage: (id, stage, error) => {
          const s = states.value[key]
          if (s) s.stages[id] = { stage, error }
        },
      })
      const s = states.value[key]
      if (s) s.outcome = outcome
      return outcome
    } finally {
      const s = states.value[key]
      if (s) {
        s.running = false
        s.cancelling = false
      }
      ctls.delete(key)
    }
  }

  function cancelMany(root: string): void {
    const key = dirKey(root)
    const ctl = ctls.get(key)
    if (!ctl || ctl.cancelled) return
    ctl.cancelled = true
    const s = states.value[key]
    if (s) s.cancelling = true
    if (ctl.session) void run.stop(ctl.session, root)
  }

  function clear(root: string): void {
    if (!isRunning(root)) delete states.value[dirKey(root)]
  }

  return { states, stateFor, isRunning, startMany, cancelMany, clear }
})
