export type QrInputError = 'empty' | 'badChars' | 'tooLong'

export const MAX_QR_LEN = 4096

export function cleanQrText(raw: string): string {
  return raw.replace(/\s+/g, '')
}

export function checkQrText(raw: string): { ok: true; text: string } | { ok: false; error: QrInputError } {
  const text = cleanQrText(raw)
  if (!text) return { ok: false, error: 'empty' }
  if (text.length > MAX_QR_LEN) return { ok: false, error: 'tooLong' }
  if (!/^[A-Za-z0-9_-]+={0,2}$/.test(text)) return { ok: false, error: 'badChars' }
  return { ok: true, text }
}

export function pairErrorKey(code: string): 'denied' | 'expired' | 'badCode' | 'noRoute' | 'failed' {
  switch (code) {
    case 'denied':
      return 'denied'
    case 'expired':
      return 'expired'
    case 'badQr':
    case 'badToken':
    case 'badKey':
    case 'badPayload':
      return 'badCode'
    case 'noRoute':
      return 'noRoute'
    default:
      return 'failed'
  }
}
