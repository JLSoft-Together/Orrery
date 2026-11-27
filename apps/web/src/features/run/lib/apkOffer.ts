import type { RunSession } from '@ccm/shared'
import { hasPushPhone, offerApkToPhone, useRemoteStore, type OfferApkResult } from '@features/remote'
import { i18n } from '../../../i18n'
import { useToastsStore } from '../../../core/stores/toasts'
import { autoApk, shouldAutoOffer } from './runApk'

const offered = new Set<string>()

export function offerToastKey(res: OfferApkResult): string {
  if (res.ok) return res.sent > 0 ? 'runApk.sent' : 'runApk.none'
  return `runApk.${res.error === 'invalid' ? 'unavailable' : res.error}`
}

export async function offerSession(sessionId: string): Promise<OfferApkResult> {
  const res = await offerApkToPhone(sessionId)
  const sent = res.ok ? res.sent : 0
  useToastsStore().push({ id: 'run-apk-offer', kind: res.ok && sent > 0 ? 'info' : 'warning', text: i18n.global.t(offerToastKey(res), { n: sent }, sent), timeoutMs: 4000 })
  return res
}

export const forgetOffered = (sessionId: string): void => void offered.delete(sessionId)

export function maybeAutoOffer(prev: RunSession | undefined, next: RunSession): void {
  if (!prev) return
  if (!shouldAutoOffer(prev, next, Object.values(autoApk.value), offered)) return
  offered.add(next.id)
  if (!hasPushPhone(useRemoteStore().devices)) return
  void offerSession(next.id)
}
