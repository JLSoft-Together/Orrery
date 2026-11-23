import { bridgeCall, type ChatCall, type ChatTarget } from './feed'

export type ExportForm = 'full' | 'short'
export type ExportDest = 'text' | 'save' | 'telegram'
export type ExportHow = 'copy' | 'save' | 'telegram'
export type ExportOutcome = 'copied' | 'saved' | 'sent' | 'cancelled' | 'failed'

export interface ExportedChat {
  text: string
  name: string
  cut: boolean
}

export interface PaneChatSource {
  key: string
  preset: string
  terminalId?: string
  agentSessionId?: string
  cwd?: string
}

export interface TermChatSource {
  id: string
  cwd?: string
  claudeSessionId?: string
  codexSessionId?: string
}

export function paneChatTarget(pane: PaneChatSource | null | undefined, term: TermChatSource | undefined): ChatTarget | null {
  if (!pane || (pane.preset !== 'claude' && pane.preset !== 'codex')) return null
  const agent = pane.preset
  const own = agent === 'claude' ? term?.claudeSessionId : (term?.codexSessionId ?? (term?.id.startsWith('codex:') ? term.id.slice(6) : undefined))
  const session = own ?? pane.agentSessionId ?? ''
  if (!session) return null
  return { key: pane.key, agent, session, cwd: term?.cwd ?? pane.cwd ?? '' }
}

function request(target: ChatTarget, form: ExportForm, dest: ExportDest): Record<string, unknown> {
  return { op: 'export', key: target.key, agent: target.agent, session: target.session, cwd: target.cwd, form, dest }
}

function record(v: unknown): Record<string, unknown> | null {
  return v && typeof v === 'object' ? (v as Record<string, unknown>) : null
}

export async function exportChat(target: ChatTarget, form: ExportForm = 'full', call: ChatCall = bridgeCall): Promise<ExportedChat | null> {
  try {
    const r = record(await call(request(target, form, 'text')))
    if (!r || r.ok !== true || typeof r.text !== 'string') return null
    return { text: r.text, name: typeof r.name === 'string' ? r.name : 'session.md', cut: r.cut === true }
  } catch {
    return null
  }
}

export interface ExportEnv {
  call?: ChatCall
  write?: (text: string) => Promise<void>
}

export async function runExport(target: ChatTarget, how: ExportHow, env: ExportEnv = {}): Promise<ExportOutcome> {
  const call = env.call ?? bridgeCall
  try {
    if (how === 'copy') {
      const got = await exportChat(target, 'full', call)
      if (!got) return 'failed'
      await (env.write ?? ((t: string) => navigator.clipboard.writeText(t)))(got.text)
      return 'copied'
    }
    const r = record(await call(request(target, 'full', how)))
    if (!r || r.ok !== true) return 'failed'
    if (how === 'save') return r.saved === true ? 'saved' : 'cancelled'
    return r.sent === true ? 'sent' : 'failed'
  } catch {
    return 'failed'
  }
}
