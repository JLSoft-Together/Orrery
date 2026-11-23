import { isPtyPreset, type PtyPreset } from '@ccm/shared'
import { dirKey } from '../../../core/stores/favorites'

export type TemplateAgent = 'claude' | 'codex'

export interface LaunchTemplate {
  id: string
  name: string
  agent: TemplateAgent
  model?: string
  permissionMode?: string
  effort?: string
  prompt?: string
}

export interface LaunchOptions {
  model?: string
  permission?: string
  effort?: string
}

export interface ProjectDefault {
  agent: PtyPreset
  template?: string
}

export const TEMPLATES_KEY = 'ccm.launchTemplates'
export const DEFAULTS_KEY = 'ccm.projectDefaults'
export const TEMPLATE_MAX = 12
export const PROMPT_MAX = 2000
export const NAME_MAX = 30

export const PERMISSIONS: Record<TemplateAgent, readonly string[]> = {
  claude: ['plan', 'acceptEdits', 'auto', 'manual', 'dontAsk'],
  codex: ['read-only', 'workspace-write'],
}

export const EFFORTS: Record<TemplateAgent, readonly string[]> = {
  claude: ['low', 'medium', 'high', 'xhigh', 'max'],
  codex: [],
}

export const isTemplateAgent = (v: unknown): v is TemplateAgent => v === 'claude' || v === 'codex'
export const isModelId = (v: unknown): v is string => typeof v === 'string' && /^[A-Za-z0-9][A-Za-z0-9._:-]{0,63}$/.test(v)

export function cleanTemplate(raw: unknown): LaunchTemplate | null {
  if (!raw || typeof raw !== 'object') return null
  const r = raw as Record<string, unknown>
  if (!isTemplateAgent(r.agent) || typeof r.id !== 'string' || !r.id || typeof r.name !== 'string') return null
  const name = r.name.trim().slice(0, NAME_MAX)
  if (!name) return null
  const agent = r.agent
  const out: LaunchTemplate = { id: r.id, name, agent }
  if (isModelId(r.model)) out.model = r.model
  if (typeof r.permissionMode === 'string' && PERMISSIONS[agent].includes(r.permissionMode)) out.permissionMode = r.permissionMode
  if (typeof r.effort === 'string' && EFFORTS[agent].includes(r.effort)) out.effort = r.effort
  if (typeof r.prompt === 'string' && r.prompt.trim()) out.prompt = r.prompt.slice(0, PROMPT_MAX)
  return out
}

export function cleanTemplates(raw: unknown): LaunchTemplate[] {
  if (!Array.isArray(raw)) return []
  const seen = new Set<string>()
  const out: LaunchTemplate[] = []
  for (const item of raw) {
    const t = cleanTemplate(item)
    if (!t || seen.has(t.id)) continue
    seen.add(t.id)
    out.push(t)
    if (out.length >= TEMPLATE_MAX) break
  }
  return out
}

export function cleanDefaults(raw: unknown): Record<string, ProjectDefault> {
  const out: Record<string, ProjectDefault> = {}
  if (!raw || typeof raw !== 'object' || Array.isArray(raw)) return out
  for (const [dir, v] of Object.entries(raw as Record<string, unknown>)) {
    if (!v || typeof v !== 'object') continue
    const d = v as Record<string, unknown>
    if (!isPtyPreset(d.agent)) continue
    out[dir] = { agent: d.agent, ...(typeof d.template === 'string' && d.template ? { template: d.template } : {}) }
  }
  return out
}

export function launchOptionsOf(t: LaunchTemplate | null | undefined): LaunchOptions | undefined {
  if (!t) return undefined
  const o: LaunchOptions = {
    ...(t.model ? { model: t.model } : {}),
    ...(t.permissionMode ? { permission: t.permissionMode } : {}),
    ...(t.effort ? { effort: t.effort } : {}),
  }
  return Object.keys(o).length ? o : undefined
}

export function templateArgsPreview(t: LaunchTemplate): string {
  const parts: string[] = [t.agent]
  if (t.model) parts.push(t.agent === 'codex' ? `-m ${t.model}` : `--model ${t.model}`)
  if (t.permissionMode) parts.push(t.agent === 'codex' ? `-s ${t.permissionMode}` : `--permission-mode ${t.permissionMode}`)
  if (t.effort) parts.push(`--effort ${t.effort}`)
  return parts.join(' ')
}

export interface Choice {
  agent: PtyPreset
  template: LaunchTemplate | null
}

export function chooseFor(
  dir: string | undefined,
  defaults: Record<string, ProjectDefault>,
  templates: readonly LaunchTemplate[],
  fallbackAgent: PtyPreset,
  enabled: (p: PtyPreset) => boolean = () => true,
): Choice {
  const d = dir ? defaults[dirKey(dir)] : undefined
  if (!d || !enabled(d.agent)) return { agent: fallbackAgent, template: null }
  const template = d.template ? (templates.find((t) => t.id === d.template && t.agent === d.agent) ?? null) : null
  return { agent: d.agent, template }
}

export function rememberChoice(defaults: Record<string, ProjectDefault>, dir: string, choice: Choice): Record<string, ProjectDefault> {
  const next = { ...defaults }
  const key = dirKey(dir)
  delete next[key]
  const entries = Object.entries(next)
  const kept = entries.length >= 50 ? Object.fromEntries(entries.slice(entries.length - 49)) : next
  return { ...kept, [key]: { agent: choice.agent, ...(choice.template ? { template: choice.template.id } : {}) } }
}

function read(key: string): unknown {
  try {
    const raw = localStorage.getItem(key)
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

function write(key: string, value: unknown): void {
  try {
    localStorage.setItem(key, JSON.stringify(value))
  } catch {
    return
  }
}

export const loadTemplates = (): LaunchTemplate[] => cleanTemplates(read(TEMPLATES_KEY))
export const saveTemplates = (list: readonly LaunchTemplate[]): void => write(TEMPLATES_KEY, list)
export const loadDefaults = (): Record<string, ProjectDefault> => cleanDefaults(read(DEFAULTS_KEY))
export const saveDefaults = (d: Record<string, ProjectDefault>): void => write(DEFAULTS_KEY, d)

export const newTemplateId = (): string => `tpl${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`
