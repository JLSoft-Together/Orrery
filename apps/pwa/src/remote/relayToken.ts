export interface RelayToken {
  token: string
  exp: number
}

function decodeBase64Url(text: string): Uint8Array | null {
  try {
    const padded = text.replace(/-/g, '+').replace(/_/g, '/').padEnd(Math.ceil(text.length / 4) * 4, '=')
    const raw = atob(padded)
    const out = new Uint8Array(raw.length)
    for (let i = 0; i < raw.length; i++) out[i] = raw.charCodeAt(i)
    return out
  } catch {
    return null
  }
}

export function parseRelayToken(token: string): RelayToken | null {
  const bytes = decodeBase64Url(token)
  if (!bytes || bytes.length < 41) return null
  const view = new DataView(bytes.buffer, bytes.byteOffset + bytes.length - 40, 8)
  const exp = Number(view.getBigUint64(0))
  return Number.isSafeInteger(exp) ? { token, exp } : null
}

function wsBase(relay: string): string {
  let base = relay.trim()
  if (base.startsWith('https://')) base = `wss://${base.slice(8)}`
  else if (base.startsWith('http://')) base = `ws://${base.slice(7)}`
  return base.replace(/\/+$/, '')
}

export function relayPhoneUrl(relay: string, machineId: string, token: string): string {
  return `${wsBase(relay)}/p/${encodeURIComponent(machineId)}?t=${encodeURIComponent(token)}`
}

export function relayPairUrl(relay: string, machineId: string, ticket: string): string {
  return `${wsBase(relay)}/p/${encodeURIComponent(machineId)}?pt=${encodeURIComponent(ticket)}`
}

export function lanUrl(host: string, port: number): string {
  const bracketed = host.includes(':') && !host.startsWith('[') ? `[${host}]` : host
  return `wss://${bracketed}:${port}/remote`
}
