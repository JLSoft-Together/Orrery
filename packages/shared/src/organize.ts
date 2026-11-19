export type OrganizeMode = 'type' | 'month'
export const ORGANIZE_MODES: readonly OrganizeMode[] = ['type', 'month']
export const ORGANIZE_KINDS = ['image', 'video', 'audio', 'doc', 'archive', 'installer', 'code', 'other'] as const
export type OrganizeKind = (typeof ORGANIZE_KINDS)[number]
export type OrganizeRuleKind = 'ext' | 'name' | 'size'
export const ORGANIZE_RULE_KINDS: readonly OrganizeRuleKind[] = ['ext', 'name', 'size']
export const ORGANIZE_MAX_RULES = 30
export const ORGANIZE_STABLE_MS = 120_000

export interface OrganizeRule {
  kind: OrganizeRuleKind
  value: string
  folder: string
}

export interface OrganizeSettings {
  sources: string[]
  source: string
  mode: OrganizeMode
  kinds: OrganizeKind[]
  rules: OrganizeRule[]
  auto: boolean
}

export interface OrganizeSettingsPatch {
  source?: string
  mode?: OrganizeMode
  kinds?: OrganizeKind[]
  rules?: OrganizeRule[]
  auto?: boolean
}

export type OrganizeSourceLabel = 'downloads' | 'desktop' | 'custom'

export interface OrganizeSource {
  path: string
  label: OrganizeSourceLabel
  ok: boolean
  reason?: string
}

export interface OrganizeRunSummary {
  id: string
  at: number
  source: string
  mode: OrganizeMode
  auto: boolean
  moved: number
  bytes: number
  undone: boolean
  undoneAt?: number
  restored?: number
  conflicts?: number
}

export interface OrganizeState {
  settings: OrganizeSettings
  sources: OrganizeSource[]
  runs: OrganizeRunSummary[]
  watching: string[]
  stableMs: number
}

export interface OrganizePlanItem {
  id: string
  name: string
  folder: string
  target: string
  size: number
  mtime: number
  kind: OrganizeKind
  rule?: number
}

export interface OrganizeSkip {
  name: string
  reason: string
}

export interface OrganizePlan {
  id: string
  source: string
  mode: OrganizeMode
  items: OrganizePlanItem[]
  skipped: OrganizeSkip[]
  error?: string
}

export interface OrganizeRunRequest {
  plan: string
  exclude?: string[]
}

export interface OrganizeRunResult {
  run: string | null
  moved: number
  bytes: number
  failed: OrganizeSkip[]
  error?: string
}

export interface OrganizeUndoResult {
  restored: number
  conflicts: OrganizeSkip[]
  dirs: number
  error?: string
}

export type OrganizeJunkKind = 'empty' | 'shortcut'
export type OrganizeJunkState = 'empty' | 'broken' | 'unknown'

export interface OrganizeJunkItem {
  path: string
  name: string
  kind: OrganizeJunkKind
  state: OrganizeJunkState
  target?: string
}

export interface OrganizeJunkResult {
  indexed: boolean
  items: OrganizeJunkItem[]
  truncated?: boolean
}
