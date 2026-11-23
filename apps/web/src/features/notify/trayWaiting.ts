import { maskText } from '../../core/lib/mask'
import { needsAnswer } from './badge'
import type { InboxReason } from '@ccm/shared'

export const TRAY_MAX_ROWS = 8
export const TRAY_MAX_TEXT = 60

export interface TrayRow {
  id: string
  agent: string
  project: string
  question: string
}

export interface TrayPayload {
  total: number
  rows: TrayRow[]
}

export interface TrayItem {
  reason: InboxReason
  terminalId?: string
  question?: string
}

export interface TrayTerminal {
  provider?: string
  cwd?: string
  title?: string
}

const SECRETS: RegExp[] = [
  /\bsk-[A-Za-z0-9_-]{16,}/g,
  /\b(?:AKIA|ASIA)[A-Z0-9]{16}\b/g,
  /\bxox[abprs]-[A-Za-z0-9-]{10,}/g,
  /\bBearer\s+[A-Za-z0-9._~+/=-]{12,}/gi,
  /\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{6,}/g,
  /\b(?:password|passwd|secret|token|api[_-]?key)\s*[:=]\s*\S+/gi,
]

export function clip(text: string, max = TRAY_MAX_TEXT): string {
  const chars = [...text]
  return chars.length <= max ? text : `${chars.slice(0, max - 1).join('').trimEnd()}…`
}

export function maskSecrets(text: string): string {
  return SECRETS.reduce((s, re) => s.replace(re, '•••'), maskText(text))
}

export function cleanQuestion(text: string | undefined): string {
  if (!text) return ''
  return clip(maskSecrets(text.replace(/\s+/g, ' ').trim()))
}

const AGENT: Record<string, string> = { claude: 'Claude', codex: 'Codex', deepseek: 'DeepSeek' }

export const folderOf = (cwd: string | undefined): string => {
  const parts = (cwd ?? '').replace(/\\/g, '/').split('/').filter(Boolean)
  return parts[parts.length - 1] ?? ''
}

export function trayPayload(items: readonly TrayItem[], byId: Readonly<Record<string, TrayTerminal | undefined>>, allows: (cwd: string | undefined) => boolean): TrayPayload {
  const seen = new Set<string>()
  const rows: TrayRow[] = []
  for (const item of items) {
    const id = item.terminalId
    if (!id || seen.has(id) || !needsAnswer(item.reason)) continue
    const x = byId[id]
    if (!allows(x?.cwd)) continue
    seen.add(id)
    rows.push({
      id,
      agent: AGENT[x?.provider ?? ''] ?? x?.provider ?? '',
      project: clip(maskSecrets(folderOf(x?.cwd)), 30),
      question: cleanQuestion(item.question),
    })
  }
  return { total: rows.length, rows: rows.slice(0, TRAY_MAX_ROWS) }
}
