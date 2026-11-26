import { expandVars, type PromptVar, type VarContext } from './promptVars'

export interface InsertTarget {
  agent: boolean
  live: boolean
  prefill?: string
  compact?: unknown
  waiting?: boolean
}

export type InsertPatch = { prefill: string; prefillSubmit: undefined; prefillNow: true }

export type InsertPlan = { ok: true; patch: InsertPatch; missing: PromptVar[]; waiting: boolean } | { ok: false; reason: 'noPane' | 'notAgent' | 'busy' }

export function planInsert(target: InsertTarget | null, body: string, ctx: VarContext): InsertPlan {
  if (!target || !target.live) return { ok: false, reason: 'noPane' }
  if (!target.agent) return { ok: false, reason: 'notAgent' }
  if (target.prefill || target.compact) return { ok: false, reason: 'busy' }
  const { text, missing } = expandVars(body, ctx)
  return { ok: true, patch: { prefill: text, prefillSubmit: undefined, prefillNow: true }, missing, waiting: !!target.waiting }
}
