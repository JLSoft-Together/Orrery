const CSI = /\x1b\[[0-?]*[ -/]*[@-~]/g
const OSC_TITLE = /\x1b\][012];[^\x07\x1b]*(?:\x07|\x1b\\)/g
const ESC_SIMPLE = /\x1b[=>]/g

export function decodeStream(chunks: Buffer[]): string {
  return new TextDecoder('utf-8').decode(Buffer.concat(chunks))
}

export function cleanOutput(text: string): string {
  return text.replace(OSC_TITLE, '').replace(CSI, '').replace(ESC_SIMPLE, '')
}

export interface Names {
  tmp: string
  ids: Map<string, string>
  pids: Map<number, string>
}

export function maskString(value: string, names: Names): string {
  let out = value.split(names.tmp).join('$TMP')
  for (const [uuid, alias] of names.ids) out = out.split(uuid).join(alias)
  return out
}

export function maskValue(value: unknown, names: Names): unknown {
  if (typeof value === 'string') return maskString(value, names)
  if (Array.isArray(value)) return value.map((v) => maskValue(v, names))
  if (value && typeof value === 'object') {
    const out: Record<string, unknown> = {}
    for (const [k, v] of Object.entries(value as Record<string, unknown>)) {
      if (k === 'pid' && typeof v === 'number') out[k] = 'PID'
      else if (k === 'rid') out[k] = 'RID'
      else if (k === 'owners' && v && typeof v === 'object') {
        const masked: Record<string, unknown> = {}
        for (const [pid, id] of Object.entries(v as Record<string, unknown>)) masked[names.pids.get(Number(pid)) ?? 'unknown-pid'] = maskValue(id, names)
        out[k] = masked
      } else out[maskString(k, names)] = maskValue(v, names)
    }
    return out
  }
  return value
}
