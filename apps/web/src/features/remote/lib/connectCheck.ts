export type CheckRelayState = 'off' | 'connecting' | 'online' | 'error'
export type CheckKey = 'lan' | 'relay' | 'push'
export type CheckTone = 'ok' | 'warn' | 'muted'

export interface CheckReply {
  lan: { ok: boolean; ms?: number }
  relay: { state: CheckRelayState; ms?: number }
  push: { ok: boolean; error?: string }
}

export interface CheckLine {
  key: CheckKey
  tone: CheckTone
  code: 'ok' | 'down' | 'none' | 'error' | 'connecting' | 'rate' | 'failed'
  ms?: number
}

export interface CheckSummary {
  lines: CheckLine[]
  warn: boolean
}

const isObj = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null
const RELAY: readonly CheckRelayState[] = ['off', 'connecting', 'online', 'error']
const ms = (v: unknown): number | undefined => (typeof v === 'number' && Number.isFinite(v) && v >= 0 ? Math.round(v) : undefined)

export function parseCheck(v: unknown): CheckReply | null {
  if (!isObj(v) || !isObj(v.lan) || !isObj(v.relay) || !isObj(v.push)) return null
  const state = RELAY.includes(v.relay.state as CheckRelayState) ? (v.relay.state as CheckRelayState) : 'error'
  return {
    lan: { ok: v.lan.ok === true, ms: ms(v.lan.ms) },
    relay: { state, ms: ms(v.relay.ms) },
    push: { ok: v.push.ok === true, error: typeof v.push.error === 'string' ? v.push.error : undefined },
  }
}

export function summarizeCheck(r: CheckReply): CheckSummary {
  const lan: CheckLine = r.lan.ok ? { key: 'lan', tone: 'ok', code: 'ok', ms: r.lan.ms } : { key: 'lan', tone: 'warn', code: 'down' }
  const relay: CheckLine =
    r.relay.state === 'online'
      ? { key: 'relay', tone: 'ok', code: 'ok', ms: r.relay.ms }
      : r.relay.state === 'off'
        ? { key: 'relay', tone: 'warn', code: 'none' }
        : r.relay.state === 'connecting'
          ? { key: 'relay', tone: 'warn', code: 'connecting' }
          : { key: 'relay', tone: 'warn', code: 'error' }
  const push: CheckLine = r.push.ok
    ? { key: 'push', tone: 'ok', code: 'ok' }
    : r.push.error === 'rate'
      ? { key: 'push', tone: 'muted', code: 'rate' }
      : r.push.error === 'none'
        ? { key: 'push', tone: 'warn', code: 'none' }
        : { key: 'push', tone: 'warn', code: 'failed' }
  const lines = [lan, relay, push]
  return { lines, warn: lines.some((l) => l.tone === 'warn') }
}

export interface CheckDeps {
  check(deviceId: string): Promise<unknown>
  sleep(ms: number): Promise<void>
}

export const CHECK_RETRY_MS = 1_200
export const CHECK_TRIES = 3

export async function runCheck(deps: CheckDeps, deviceId: string, signal: AbortSignal): Promise<CheckSummary | null> {
  for (let i = 0; i < CHECK_TRIES; i++) {
    if (signal.aborted) return null
    const reply = parseCheck(await deps.check(deviceId).catch(() => null))
    if (reply) return summarizeCheck(reply)
    if (i < CHECK_TRIES - 1) await deps.sleep(CHECK_RETRY_MS)
  }
  return null
}

export const unknownCheck = (): CheckSummary => ({
  lines: (['lan', 'relay', 'push'] as const).map((key): CheckLine => ({ key, tone: 'muted', code: 'failed' })),
  warn: true,
})

export const isUnknownCheck = (s: CheckSummary): boolean => s.lines.every((l) => l.tone === 'muted' && l.code === 'failed')
