import type { RemoteGuardOp, RemoteGuardReply, RemoteGuardState } from '@ccm/shared'
import { desktop } from './desktop'

export const hasRemoteGuard = typeof desktop?.remoteGuard === 'function'

export const GUARD_TTLS: readonly (number | null)[] = [3600, 86_400, 7 * 86_400, 30 * 86_400, null]

export const emptyGuard = (): RemoteGuardState => ({ blocked: [], trusted: [], log: [], rev: 0 })

export async function remoteGuard(op: RemoteGuardOp): Promise<RemoteGuardReply | null> {
  return (await desktop?.remoteGuard?.(op).catch(() => null)) ?? null
}

const RULE = /^[0-9A-Fa-f:.]{2,45}(\/\d{1,3})?$/

export function looksLikeRule(text: string): boolean {
  const t = text.trim()
  if (!RULE.test(t)) return false
  const [addr = '', bits] = t.split('/')
  const v6 = addr.includes(':')
  if (!v6 && !/^\d{1,3}(\.\d{1,3}){3}$/.test(addr)) return false
  if (bits === undefined) return true
  const n = Number(bits)
  return v6 ? n >= 16 && n <= 128 : n >= 8 && n <= 32
}

export function splitRules(text: string): string[] {
  return [...new Set(text.split(/[\s,;]+/).map((s) => s.trim()).filter(Boolean))]
}
