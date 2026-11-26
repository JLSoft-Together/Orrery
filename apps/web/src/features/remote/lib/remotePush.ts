import type { RemoteDevice } from '@ccm/shared'
import { desktop } from '../../../core/lib/desktop'

export type SendPaneError = 'off' | 'invalid' | 'notFound' | 'none' | 'limit' | 'unavailable'
export type SendPaneResult = { ok: true; sent: number } | { ok: false; error: SendPaneError }

interface PushBridge {
  remoteSendPane?: (paneKey: string) => Promise<unknown>
}

const ERRORS: readonly SendPaneError[] = ['off', 'invalid', 'notFound', 'none', 'limit']
const UNAVAILABLE: SendPaneResult = { ok: false, error: 'unavailable' }

export function parseSendPane(raw: unknown): SendPaneResult {
  if (typeof raw !== 'object' || raw === null) return UNAVAILABLE
  const { ok, sent, error } = raw as { ok?: unknown; sent?: unknown; error?: unknown }
  if (ok === true) return { ok: true, sent: typeof sent === 'number' && Number.isFinite(sent) && sent >= 0 ? Math.floor(sent) : 0 }
  const known = ERRORS.find((e) => e === error)
  return { ok: false, error: known ?? 'unavailable' }
}

export async function sendPaneToPhone(paneKey: string): Promise<SendPaneResult> {
  const send = (desktop as unknown as PushBridge | undefined)?.remoteSendPane
  if (typeof send !== 'function' || !paneKey) return UNAVAILABLE
  try {
    return parseSendPane(await send(paneKey))
  } catch {
    return UNAVAILABLE
  }
}

export function hasPushPhone(devices: readonly Pick<RemoteDevice, 'push' | 'pushOn' | 'revoking'>[] | undefined): boolean {
  return (devices ?? []).some((d) => (d.push === 'fcm' || d.push === 'web') && d.pushOn !== false && !d.revoking)
}
