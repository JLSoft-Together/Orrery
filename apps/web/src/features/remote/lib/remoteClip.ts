import { desktop } from '../../../core/lib/desktop'

export type ClipError = 'off' | 'noSession' | 'empty' | 'limit' | 'secret' | 'internal' | 'unavailable'
export type ClipResult = { ok: true; sent: number } | { ok: false; error: ClipError }

interface ClipBridge {
  remoteClipSend?: () => Promise<unknown>
}

const ERRORS: readonly ClipError[] = ['off', 'noSession', 'empty', 'limit', 'secret', 'internal']
const UNAVAILABLE: ClipResult = { ok: false, error: 'unavailable' }

export function parseClipSend(raw: unknown): ClipResult {
  if (typeof raw !== 'object' || raw === null) return UNAVAILABLE
  const { ok, sent, error } = raw as { ok?: unknown; sent?: unknown; error?: unknown }
  if (ok === true) return { ok: true, sent: typeof sent === 'number' && Number.isFinite(sent) && sent >= 0 ? Math.floor(sent) : 0 }
  return { ok: false, error: ERRORS.find((e) => e === error) ?? 'unavailable' }
}

export async function sendClipboardToPhone(): Promise<ClipResult> {
  const send = (desktop as unknown as ClipBridge | undefined)?.remoteClipSend
  if (typeof send !== 'function') return UNAVAILABLE
  try {
    return parseClipSend(await send())
  } catch {
    return UNAVAILABLE
  }
}

export function clipToastKey(res: ClipResult): string {
  return res.ok ? (res.sent > 0 ? 'remoteClip.sent' : 'remoteClip.noSession') : `remoteClip.${res.error}`
}
