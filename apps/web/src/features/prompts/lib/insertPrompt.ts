import { expandVars, type PromptVar, type VarContext } from './promptVars'

export interface InsertTarget {
  agent: boolean
  live: boolean
  prefill?: string
  compact?: unknown
  status?: string
}

export type InsertPatch = { prefill: string; prefillSubmit: undefined; prefillNow: true }

export type InsertPlan = { ok: true; patch: InsertPatch; missing: PromptVar[] } | { ok: false; reason: 'noPane' | 'notAgent' | 'busy' | 'waiting' }

export function planInsert(target: InsertTarget | null, body: string, ctx: VarContext): InsertPlan {
  if (!target || !target.live) return { ok: false, reason: 'noPane' }
  if (!target.agent) return { ok: false, reason: 'notAgent' }
  if (target.prefill || target.compact) return { ok: false, reason: 'busy' }
  if (target.status === 'waiting') return { ok: false, reason: 'waiting' }
  if (target.status && target.status !== 'idle' && target.status !== 'working') return { ok: false, reason: 'busy' }
  const { text, missing } = expandVars(body, ctx)
  return { ok: true, patch: { prefill: text, prefillSubmit: undefined, prefillNow: true }, missing }
}
