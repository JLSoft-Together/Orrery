export type TermMode = 'desk' | 'read'

export type ModeAction = 'toggle' | 'input'

export interface ReadLine {
  translateToString(trimRight?: boolean): string
  readonly isWrapped?: boolean
}

export interface ReadBuffer {
  readonly length: number
  getLine(y: number): ReadLine | undefined
}

const PREFIX = 'ccm.pwa.termMode.'

export function nextMode(mode: TermMode, action: ModeAction): TermMode {
  if (action === 'input') return 'desk'
  return mode === 'desk' ? 'read' : 'desk'
}

export function bufferParagraphs(b: ReadBuffer): string[] {
  const out: string[] = []
  for (let i = 0; i < b.length; i++) {
    const line = b.getLine(i)
    const text = line?.translateToString(false) ?? ''
    if (line?.isWrapped && out.length > 0) out[out.length - 1] += text
    else out.push(text)
  }
  return out.map((l) => l.replace(/\s+$/, ''))
}

export function readerText(b: ReadBuffer): string {
  const lines = bufferParagraphs(b)
  let end = lines.length
  while (end > 0 && lines[end - 1] === '') end--
  return lines.slice(0, end).join('\n')
}

export function loadMode(sessionId: string | null, storage: Pick<Storage, 'getItem'> | null = safeStorage()): TermMode {
  if (!sessionId) return 'desk'
  try {
    return storage?.getItem(PREFIX + sessionId) === 'read' ? 'read' : 'desk'
  } catch {
    return 'desk'
  }
}

export function saveMode(sessionId: string | null, mode: TermMode, storage: Pick<Storage, 'setItem' | 'removeItem'> | null = safeStorage()): void {
  if (!sessionId) return
  try {
    if (mode === 'read') storage?.setItem(PREFIX + sessionId, 'read')
    else storage?.removeItem(PREFIX + sessionId)
  } catch {
    return
  }
}

function safeStorage(): Storage | null {
  try {
    return typeof localStorage === 'undefined' ? null : localStorage
  } catch {
    return null
  }
}
