import { isAgentSessionId, type PtyPreset, type TerminalSession } from '@ccm/shared'
import { dirKey } from '../stores/favorites'
import { sessionIdOf } from './mapText'

export interface ResumeItem {
  id: string
  terminalId: string
  name: string
  cwd: string
  endedAt: string
}

export const LAUNCH_AGENTS: readonly PtyPreset[] = ['claude', 'codex', 'deepseek', 'shell']

export function launchAgents(deepseek: boolean): PtyPreset[] {
  return LAUNCH_AGENTS.filter((p) => p !== 'deepseek' || deepseek)
}

export const stepIndex = (at: number, step: number, len: number): number => (len <= 0 ? -1 : (((at + step) % len) + len) % len)

const within = (cwd: string, dir: string): boolean => {
  const c = dirKey(cwd)
  const d = dirKey(dir)
  return c === d || c.startsWith(`${d}\\`)
}

const baseOf = (path: string): string => path.split(/[\\/]+/).filter(Boolean).pop() ?? path

export function launchResumables(
  list: readonly TerminalSession[],
  dir: string | undefined,
  preset: PtyPreset,
  openIds: ReadonlySet<string>,
  max = 5,
): ResumeItem[] {
  if (!dir || preset === 'shell') return []
  const seen = new Set<string>()
  return list
    .filter((t) => {
      if (!t.endedAt || !t.cwd || (t.provider ?? 'claude') !== preset || openIds.has(t.id)) return false
      const id = sessionIdOf(t)
      return isAgentSessionId(id) && within(t.cwd, dir)
    })
    .sort((a, b) => (b.endedAt ?? '').localeCompare(a.endedAt ?? ''))
    .filter((t) => {
      const id = sessionIdOf(t)!
      if (seen.has(id)) return false
      seen.add(id)
      return true
    })
    .slice(0, max)
    .map((t) => ({ id: sessionIdOf(t)!, terminalId: t.id, name: t.alias || baseOf(t.cwd!), cwd: t.cwd!, endedAt: t.endedAt! }))
}
