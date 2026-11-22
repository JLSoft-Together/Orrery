import {
  ORGANIZE_KINDS,
  ORGANIZE_MAX_RULES,
  ORGANIZE_MODES,
  ORGANIZE_RULE_KINDS,
  type DiskDeleteResult,
  type OrganizeJunkItem,
  type OrganizeJunkResult,
  type OrganizeKind,
  type OrganizeMode,
  type OrganizePlan,
  type OrganizePlanItem,
  type OrganizeRule,
  type OrganizeRuleKind,
  type OrganizeRunResult,
  type OrganizeRunSummary,
  type OrganizeSettingsPatch,
  type OrganizeSource,
  type OrganizeState,
  type OrganizeUndoResult,
} from '@ccm/shared'
import { desktop } from '../../../core/lib/desktop'

export type { OrganizeJunkItem, OrganizeJunkResult, OrganizeKind, OrganizeMode, OrganizePlan, OrganizePlanItem, OrganizeRule, OrganizeRuleKind, OrganizeRunResult, OrganizeRunSummary, OrganizeSource, OrganizeState, OrganizeUndoResult }
export { ORGANIZE_KINDS, ORGANIZE_MAX_RULES, ORGANIZE_MODES, ORGANIZE_RULE_KINDS }

interface OrganizeBridge {
  organizeState?: () => Promise<unknown>
  organizeSettings?: (patch: unknown) => Promise<unknown>
  organizeAddSource?: (kind: unknown) => Promise<unknown>
  organizeRemoveSource?: (path: unknown) => Promise<unknown>
  organizePlan?: (req: unknown) => Promise<unknown>
  organizeRun?: (req: unknown) => Promise<unknown>
  organizeUndo?: (req: unknown) => Promise<unknown>
  organizeJunk?: () => Promise<unknown>
  onOrganizeChanged?: (cb: (p: unknown) => void) => () => void
  diskMapDelete?: (req: unknown) => Promise<unknown>
}
const bridge = desktop as unknown as OrganizeBridge | undefined

export const hasOrganize = typeof bridge?.organizePlan === 'function' && typeof bridge?.organizeRun === 'function'

const call = async <T>(fn: (() => Promise<unknown>) | undefined, fallback: T): Promise<T> => {
  if (!fn) return fallback
  try {
    const v = await fn()
    return (v ?? fallback) as T
  } catch {
    return fallback
  }
}

export const organizeState = (): Promise<OrganizeState | null> => call<OrganizeState | null>(bridge?.organizeState && (() => bridge.organizeState!()), null)
export const organizeSettings = (patch: OrganizeSettingsPatch): Promise<OrganizeState | null> =>
  call<OrganizeState | null>(bridge?.organizeSettings && (() => bridge.organizeSettings!(JSON.parse(JSON.stringify(patch)))), null)
export const organizeAddSource = (kind: 'downloads' | 'desktop' | 'pick'): Promise<{ state: OrganizeState; reason?: string } | null> =>
  call<{ state: OrganizeState; reason?: string } | null>(bridge?.organizeAddSource && (() => bridge.organizeAddSource!(kind)), null)
export const organizeRemoveSource = (p: string): Promise<OrganizeState | null> => call<OrganizeState | null>(bridge?.organizeRemoveSource && (() => bridge.organizeRemoveSource!(p)), null)
export const organizePlan = (source: string): Promise<OrganizePlan | null> => call<OrganizePlan | null>(bridge?.organizePlan && (() => bridge.organizePlan!({ source })), null)
export const organizeRun = (plan: string, exclude: string[]): Promise<OrganizeRunResult | null> =>
  call<OrganizeRunResult | null>(bridge?.organizeRun && (() => bridge.organizeRun!({ plan, exclude: [...exclude] })), null)
export const organizeUndo = (run: string): Promise<OrganizeUndoResult | null> => call<OrganizeUndoResult | null>(bridge?.organizeUndo && (() => bridge.organizeUndo!({ run })), null)
export const organizeJunk = (): Promise<OrganizeJunkResult> => call<OrganizeJunkResult>(bridge?.organizeJunk && (() => bridge.organizeJunk!()), { indexed: false, items: [] })
export const organizeDelete = (paths: string[]): Promise<DiskDeleteResult | null> =>
  call<DiskDeleteResult | null>(bridge?.diskMapDelete && (() => bridge.diskMapDelete!({ paths: [...paths], permanent: false })), null)
export const onOrganizeChanged = (cb: () => void): (() => void) => bridge?.onOrganizeChanged?.(() => cb()) ?? (() => {})

export function baseName(p: string): string {
  const parts = p.split(/[\\/]/).filter(Boolean)
  return parts[parts.length - 1] ?? p
}

export function junkPreselect(items: OrganizeJunkItem[]): Set<string> {
  return new Set(items.filter((i) => i.state !== 'unknown').map((i) => i.path))
}

export function emptyRule(kind: OrganizeRuleKind = 'ext'): OrganizeRule {
  return { kind, value: '', folder: '' }
}

export function ruleValid(r: OrganizeRule): boolean {
  const folder = String(r.folder ?? '').trim()
  if (!folder || folder.length > 64 || /[<>:"/\\|?*]|[.]$/.test(folder) || folder === '..') return false
  const v = String(r.value ?? '').trim()
  if (r.kind === 'ext') return /^\*?\.?[a-zA-Z0-9_!-]{1,16}$/.test(v)
  if (r.kind === 'size') return Number.isFinite(Number(v)) && Number(v) > 0
  return v.length > 0 && v.length <= 64
}
