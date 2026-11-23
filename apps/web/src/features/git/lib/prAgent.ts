import type { ChatTarget } from '../../chat/feed'
import { exportChat, paneChatTarget } from '../../chat/export'
import { sendTargets } from '../../files/lib/sendSelection'
import { presetOf, workCwd, type Pane } from '../../workspace/stores/workspace'

export interface TermLike {
  id: string
  cwd?: string
  claudeSessionId?: string
  codexSessionId?: string
}

export function agentPanes(panes: readonly Pane[], root: string): Pane[] {
  return sendTargets(panes, root, root)
}

export function chatTargetOf(pane: Pane, byId: Record<string, TermLike | undefined>): ChatTarget | null {
  return paneChatTarget({ key: pane.key, preset: presetOf(pane), terminalId: pane.terminalId, agentSessionId: pane.agentSessionId, cwd: workCwd(pane) }, pane.terminalId ? byId[pane.terminalId] : undefined)
}

export async function loadPrBody(panes: readonly Pane[], root: string, byId: Record<string, TermLike | undefined>): Promise<string | null> {
  for (const pane of agentPanes(panes, root)) {
    const target = chatTargetOf(pane, byId)
    if (!target) continue
    const got = await exportChat(target, 'short')
    if (got?.text.trim()) return got.text
  }
  return null
}

function fence(text: string): string {
  const longest = Math.max(0, ...(text.match(/`+/g) ?? []).map((r) => r.length))
  return '`'.repeat(Math.max(3, longest + 1))
}

export function errorPrompt(head: string, job: string, branch: string, lines: readonly string[]): string {
  const body = lines.join('\n').replace(/\r\n|\r/g, '\n').replace(/\n+$/, '')
  const f = fence(body)
  return `${head} ${job} (${branch})\n${f}\n${body}\n${f}\n`
}
