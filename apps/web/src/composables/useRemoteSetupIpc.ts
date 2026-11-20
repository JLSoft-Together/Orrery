import type { PushSetupResult, PushStatus, PushTestResult, RelayAccount, RelayDeployProgress, RelaySetupError, RelaySetupStatus } from '@ccm/shared'
import { desktop } from '../lib/desktop'

export type RelayErrorCode = RelaySetupError | 'cancelled' | 'migration'

export type RelayTokenResult = { ok: true; accounts: RelayAccount[] } | { ok: false; error: RelayErrorCode }
export type RelayDeployResult = { ok: true; url: string; warning?: 'health' } | { ok: false; error: RelayErrorCode }
export type RelayQuickResult =
  | { ok: true; url: string; warning?: 'health' }
  | { ok: false; error: RelayErrorCode | 'pick_account'; accounts?: RelayAccount[]; detail?: string; link?: string }
export type RelayRemoveResult = { ok: true; workerDeleted: boolean } | { ok: false; error: RelayErrorCode; workerDeleted?: boolean }

interface SetupBridge {
  remoteRelayStatus?: () => Promise<unknown>
  remoteRelayToken?: (token: string) => Promise<unknown>
  remoteRelayDeploy?: (accountId: string) => Promise<unknown>
  remoteRelayRemove?: (deleteWorker: boolean) => Promise<unknown>
  remoteRelayQuick?: (accountId: string) => Promise<unknown>
  remoteRelayCancel?: () => Promise<unknown>
  remoteRelayClipboard?: () => Promise<unknown>
  onRemoteRelayProgress?: (handler: (progress: unknown) => void) => () => void
  remotePushStatus?: () => Promise<unknown>
  remotePushSetup?: (googleServices: string, serviceAccount: string) => Promise<unknown>
  remotePushTest?: (deviceId?: string) => Promise<unknown>
  remotePushRemove?: () => Promise<unknown>
}

const seam = (window as unknown as { __ccmE2eSetupBridge?: SetupBridge }).__ccmE2eSetupBridge
const bridge = seam ?? (desktop as unknown as SetupBridge | undefined)

export const hasRelaySetup = typeof bridge?.remoteRelayStatus === 'function'
export const hasRelayQuick = typeof bridge?.remoteRelayQuick === 'function'
export const hasPushSetup = typeof bridge?.remotePushStatus === 'function'

const isObj = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null
const code = (v: unknown, fallback: string): string => (typeof v === 'string' && v ? v : fallback)

export async function relayStatus(): Promise<RelaySetupStatus | null> {
  const r = await bridge?.remoteRelayStatus?.().catch(() => null)
  return isObj(r) && typeof r.state === 'string' ? (r as unknown as RelaySetupStatus) : null
}

export async function relayToken(token: string): Promise<RelayTokenResult> {
  const r = await bridge?.remoteRelayToken?.(token).catch(() => null)
  if (!isObj(r)) return { ok: false, error: 'rejected' }
  if (r.ok === true && Array.isArray(r.accounts)) {
    const accounts = r.accounts.filter((a): a is RelayAccount => isObj(a) && typeof a.id === 'string' && typeof a.name === 'string')
    return { ok: true, accounts }
  }
  return { ok: false, error: code(r.error, 'rejected') as RelayErrorCode }
}

export async function relayDeploy(accountId: string): Promise<RelayDeployResult> {
  const r = await bridge?.remoteRelayDeploy?.(accountId).catch(() => null)
  if (!isObj(r)) return { ok: false, error: 'rejected' }
  if (r.ok === true && typeof r.url === 'string') return { ok: true, url: r.url, ...(r.warning === 'health' ? { warning: 'health' as const } : {}) }
  return { ok: false, error: code(r.error, 'rejected') as RelayErrorCode }
}

const ONBOARDING = /^https:\/\/dash\.cloudflare\.com\/[0-9a-f]{32}\/workers\/onboarding$/
const DETAIL_MAX = 600

export function parseAccounts(v: unknown): RelayAccount[] {
  return Array.isArray(v) ? v.filter((a): a is RelayAccount => isObj(a) && typeof a.id === 'string' && typeof a.name === 'string') : []
}

export function parseQuick(r: unknown): RelayQuickResult {
  if (!isObj(r)) return { ok: false, error: 'rejected' }
  if (r.ok === true && typeof r.url === 'string') return { ok: true, url: r.url, ...(r.warning === 'health' ? { warning: 'health' as const } : {}) }
  const out: RelayQuickResult = { ok: false, error: code(r.error, 'rejected') as RelayErrorCode }
  const accounts = parseAccounts(r.accounts)
  if (accounts.length) out.accounts = accounts
  if (typeof r.detail === 'string' && r.detail) out.detail = r.detail.slice(0, DETAIL_MAX)
  if (typeof r.link === 'string' && ONBOARDING.test(r.link)) out.link = r.link
  return out
}

export async function relayQuick(accountId: string): Promise<RelayQuickResult> {
  return parseQuick(await bridge?.remoteRelayQuick?.(accountId).catch(() => null))
}

export async function relayCancel(): Promise<boolean> {
  const r = await bridge?.remoteRelayCancel?.().catch(() => null)
  return isObj(r) && r.ok === true
}

export async function relayClipboard(): Promise<RelayTokenResult | null> {
  const r = await bridge?.remoteRelayClipboard?.().catch(() => null)
  if (!isObj(r) || r.error === 'none') return null
  if (r.ok === true) return { ok: true, accounts: parseAccounts(r.accounts) }
  return { ok: false, error: code(r.error, 'rejected') as RelayErrorCode }
}

export async function relayRemove(deleteWorker: boolean): Promise<RelayRemoveResult> {
  const r = await bridge?.remoteRelayRemove?.(deleteWorker).catch(() => null)
  if (!isObj(r)) return { ok: false, error: 'rejected' }
  if (r.ok === true) return { ok: true, workerDeleted: r.workerDeleted === true }
  return { ok: false, error: code(r.error, 'rejected') as RelaySetupError, workerDeleted: r.workerDeleted === true }
}

export const onRelayProgress = (handler: (progress: RelayDeployProgress) => void): (() => void) =>
  bridge?.onRemoteRelayProgress?.((p) => {
    if (isObj(p) && typeof p.step === 'string') handler(p as unknown as RelayDeployProgress)
  }) ?? (() => undefined)

export async function pushStatus(): Promise<PushStatus | null> {
  const r = await bridge?.remotePushStatus?.().catch(() => null)
  return isObj(r) && typeof r.configured === 'boolean' ? (r as unknown as PushStatus) : null
}

export async function pushSetup(googleServices: string, serviceAccount: string): Promise<PushSetupResult> {
  const r = await bridge?.remotePushSetup?.(googleServices, serviceAccount).catch(() => null)
  if (!isObj(r)) return { ok: false, error: 'bad_json' }
  if (r.ok === true && typeof r.projectId === 'string') return { ok: true, projectId: r.projectId }
  return { ok: false, error: code(r.error, 'bad_json') as never }
}

export async function pushTest(deviceId?: string): Promise<PushTestResult> {
  const r = await bridge?.remotePushTest?.(deviceId).catch(() => null)
  if (!isObj(r)) return { ok: false, error: 'unavailable' }
  if (r.ok === true) return { ok: true, sent: typeof r.sent === 'number' ? r.sent : 0 }
  return { ok: false, error: code(r.error, 'unavailable') as never }
}

export async function pushRemove(): Promise<boolean> {
  const r = await bridge?.remotePushRemove?.().catch(() => null)
  return isObj(r) ? r.ok !== false : r === true
}
