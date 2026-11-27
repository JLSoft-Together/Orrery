import { desktop } from '../../../core/lib/desktop'

export type OfferApkError = 'off' | 'invalid' | 'notFound' | 'noApk' | 'gone' | 'outside' | 'notApk' | 'none' | 'lan' | 'limit' | 'offline' | 'unavailable'
export type OfferApkResult = { ok: true; sent: number } | { ok: false; error: OfferApkError }

interface OfferBridge {
  remoteOfferApk?: (sessionId: string) => Promise<unknown>
}

const ERRORS: readonly OfferApkError[] = ['off', 'invalid', 'notFound', 'noApk', 'gone', 'outside', 'notApk', 'none', 'lan', 'limit', 'offline']
const UNAVAILABLE: OfferApkResult = { ok: false, error: 'unavailable' }

export function parseOfferApk(raw: unknown): OfferApkResult {
  if (typeof raw !== 'object' || raw === null) return UNAVAILABLE
  const { ok, sent, error } = raw as { ok?: unknown; sent?: unknown; error?: unknown }
  if (ok === true) return { ok: true, sent: typeof sent === 'number' && Number.isFinite(sent) && sent >= 0 ? Math.floor(sent) : 0 }
  return { ok: false, error: ERRORS.find((e) => e === error) ?? 'unavailable' }
}

export async function offerApkToPhone(sessionId: string): Promise<OfferApkResult> {
  const offer = (desktop as unknown as OfferBridge | undefined)?.remoteOfferApk
  if (typeof offer !== 'function' || !sessionId) return UNAVAILABLE
  try {
    return parseOfferApk(await offer(sessionId))
  } catch {
    return UNAVAILABLE
  }
}
