import { dirKey } from '../../../core/stores/favorites'
import { MASK, maskToken } from '../../../core/lib/mask'
import { fileMention } from '../../agent-tools/lib/promptAttachments'
import { isTerm, presetOf, type Pane } from '../../workspace/stores/workspace'

export const SEND_MAX_LINES = 200

export interface SelectionSend {
  text: string
  full: boolean
}

const PRIVATE_KEY = /-----BEGIN [A-Z ]*PRIVATE KEY-----[\s\S]*?(?:-----END [A-Z ]*PRIVATE KEY-----|$)/g
const TOKENS = /\b(?:AKIA[0-9A-Z]{16}|sk-[A-Za-z0-9_-]{20,}|xox[abprs]-[A-Za-z0-9-]{10,}|AIza[0-9A-Za-z_-]{35})\b/g
const BEARER = /\b(Bearer|Basic)\s+[A-Za-z0-9._~+/=-]{8,}/gi
const ASSIGN = /((?:api[_-]?key|secret|token|passw(?:or)?d|passwd|private[_-]?key|access[_-]?key|credential)\w*["']?\s*[:=]\s*)("[^"\n]{4,}"|'[^'\n]{4,}'|[A-Za-z0-9+/_.~-]{12,}(?=$|[\s,;]))/gim

function maskAssign(_m: string, head: string, value: string): string {
  const q = value[0]
  if (q === '"' || q === "'") return `${head}${q}${MASK}${q}`
  return /\d/.test(value) ? `${head}${MASK}` : `${head}${value}`
}

export function maskSecrets(code: string): string {
  return maskToken(code)
    .replace(PRIVATE_KEY, MASK)
    .replace(TOKENS, MASK)
    .replace(BEARER, `$1 ${MASK}`)
    .replace(ASSIGN, maskAssign)
}

export function lineRange(from: number, to: number): string {
  return from === to ? `${from}` : `${from}-${to}`
}

function fenceFor(code: string): string {
  const longest = Math.max(0, ...(code.match(/`+/g) ?? []).map((r) => r.length))
  return '`'.repeat(Math.max(3, longest + 1))
}

export function formatSelection(input: { path: string; cwd: string | undefined; win: boolean; from: number; to: number; code: string; lang?: string | null }): SelectionSend {
  const from = Math.min(input.from, input.to)
  const to = Math.max(input.from, input.to)
  const mention = `${fileMention(input.path, input.cwd, input.win)}:${lineRange(from, to)}`
  if (to - from + 1 > SEND_MAX_LINES) return { text: `${mention} `, full: false }
  const code = maskSecrets(input.code.replace(/\r\n|\r/g, '\n').replace(/\n+$/, ''))
  const fence = fenceFor(code)
  return { text: `${mention}\n${fence}${input.lang ?? ''}\n${code}\n${fence}\n`, full: true }
}

const AGENTS = new Set(['claude', 'codex', 'deepseek'])

const within = (dir: string, root: string): boolean => {
  const d = dirKey(dir)
  const r = dirKey(root)
  return d === r || d.startsWith(`${r}\\`)
}

export function sendTargets(panes: readonly Pane[], filePath: string, project: string | undefined): Pane[] {
  return panes
    .filter((p) => isTerm(p) && AGENTS.has(presetOf(p)) && p.state === 'running' && !!p.cwd && (within(filePath, p.cwd) || (!!project && within(p.cwd, project))))
    .sort((a, b) => (b.focusedAt ?? 0) - (a.focusedAt ?? 0))
}
