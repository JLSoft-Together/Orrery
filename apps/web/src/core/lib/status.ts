import type { AgentStatus, TerminalStatus } from '@ccm/shared'

export type AnyStatus = AgentStatus | TerminalStatus

export type StatusGlyph = 'spinner' | 'half' | 'ring' | 'check' | 'alert' | 'dash' | 'cross' | 'question'

export interface StatusMeta {
  glyph: StatusGlyph
  labelKey: string
  text: string
  soft: string
  chip: string
  border: string
  bar: string
}

const META: Record<AnyStatus, StatusMeta> = {
  working: { glyph: 'spinner', labelKey: 'status.working', text: 'text-st-working', soft: 'bg-st-working-soft', chip: 'ccm-chip ccm-chip-working', border: 'border-st-working', bar: 'bg-st-working' },
  waiting: { glyph: 'half', labelKey: 'status.waiting', text: 'text-st-waiting', soft: 'bg-st-waiting-soft', chip: 'ccm-chip ccm-chip-waiting', border: 'border-st-waiting', bar: 'bg-st-waiting' },
  idle: { glyph: 'ring', labelKey: 'status.idle', text: 'text-st-idle', soft: 'bg-raised', chip: 'ccm-chip ccm-chip-quiet', border: 'border-line-strong', bar: 'bg-transparent' },
  completed: { glyph: 'check', labelKey: 'status.completed', text: 'text-st-done', soft: 'bg-raised', chip: 'ccm-chip ccm-chip-quiet', border: 'border-line', bar: 'bg-transparent' },
  error: { glyph: 'alert', labelKey: 'status.error', text: 'text-st-error', soft: 'bg-st-error-soft', chip: 'ccm-chip ccm-chip-error', border: 'border-st-error', bar: 'bg-st-error' },
  cancelled: { glyph: 'cross', labelKey: 'status.cancelled', text: 'text-st-stale', soft: 'bg-raised', chip: 'ccm-chip ccm-chip-quiet', border: 'border-line', bar: 'bg-transparent' },
  stale: { glyph: 'dash', labelKey: 'status.stale', text: 'text-st-stale', soft: 'bg-raised', chip: 'ccm-chip ccm-chip-quiet', border: 'border-line', bar: 'bg-transparent' },
  unknown: { glyph: 'question', labelKey: 'status.unknown', text: 'text-st-idle', soft: 'bg-raised', chip: 'ccm-chip ccm-chip-quiet', border: 'border-line', bar: 'bg-transparent' },
}

export function statusMeta(status: AnyStatus | string | undefined): StatusMeta {
  return META[(status ?? 'unknown') as AnyStatus] ?? META.unknown
}

export function isQuiet(status: AnyStatus): boolean {
  return status === 'completed' || status === 'cancelled' || status === 'stale'
}
