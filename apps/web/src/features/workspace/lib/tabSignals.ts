export type TabSignalKind = 'waiting' | 'error' | 'working'
export type TabExtra = 'peer' | 'telegram' | 'dirty' | 'zoom' | 'done' | 'ended'

export interface TabSignalInput {
  status?: string | null
  attention?: 'waiting' | 'done'
  shellFailed?: boolean
  peer?: boolean
  telegram?: boolean
  dirty?: boolean
  zoom?: boolean
  ended?: boolean
}

export function tabSignal(i: TabSignalInput): TabSignalKind | null {
  if (i.attention === 'waiting' || i.status === 'waiting') return 'waiting'
  if (i.status === 'error' || (i.attention === 'done' && i.shellFailed)) return 'error'
  if (i.status === 'working') return 'working'
  return null
}

export function tabExtras(i: TabSignalInput): TabExtra[] {
  const out: TabExtra[] = []
  if (i.attention === 'done' && !i.shellFailed) out.push('done')
  if (i.peer) out.push('peer')
  if (i.telegram) out.push('telegram')
  if (i.dirty) out.push('dirty')
  if (i.zoom) out.push('zoom')
  if (i.ended) out.push('ended')
  return out
}

export function tabSignalLines(signal: TabSignalKind | null, extras: readonly TabExtra[], text: (k: TabSignalKind | TabExtra) => string): string[] {
  const out: string[] = []
  if (signal) out.push(text(signal))
  for (const e of extras) out.push(text(e))
  return out.filter(Boolean)
}
