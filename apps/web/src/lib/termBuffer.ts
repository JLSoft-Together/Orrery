export const TERM_SCROLLBACK = 3000
export const HIDDEN_MAX_BYTES = 2 * 1024 * 1024
export const HIDDEN_FLUSH_MS = 1000

const ESC = 0x1b
const CARRY = 31
const REPLY_WANTED = /\x1b\[[>=?]?[\d;]*[cn]|\x1b\[\??[\d;]*\$p|\x1bP[$+]q|\x1b\]\d+;(?:\d+;)?\?|\x1b\[\d*t|\x1b\[\?u|\x1b\[>\d*q/
const REPLY_ALL = new RegExp(REPLY_WANTED.source, 'g')

export interface HiddenOutputOptions {
  write: (bytes: Uint8Array, done?: () => void) => void
  max?: number
  flushMs?: number
  later?: (fn: () => void, ms: number) => () => void
}

const defaultLater = (fn: () => void, ms: number): (() => void) => {
  const timer = setTimeout(fn, ms)
  return () => clearTimeout(timer)
}

const latin1 = new TextDecoder('latin1')

export class HiddenOutput {
  private chunks: Uint8Array[] = []
  private size = 0
  private shown = true
  private carry = ''
  private cancel: (() => void) | null = null
  private readonly max: number
  private readonly flushMs: number
  private readonly later: (fn: () => void, ms: number) => () => void

  constructor(private readonly opts: HiddenOutputOptions) {
    this.max = opts.max ?? HIDDEN_MAX_BYTES
    this.flushMs = opts.flushMs ?? HIDDEN_FLUSH_MS
    this.later = opts.later ?? defaultLater
  }

  get pending(): number {
    return this.size
  }

  get visible(): boolean {
    return this.shown
  }

  setVisible(visible: boolean, done?: () => void): void {
    this.shown = visible
    if (visible) this.flush(done)
    else done?.()
  }

  push(bytes: Uint8Array): void {
    if (!bytes.length) return
    if (this.shown) {
      this.flush()
      this.keepTail(bytes)
      this.opts.write(bytes)
      return
    }
    this.chunks.push(bytes)
    this.size += bytes.length
    if (this.wantsReply(bytes) || this.size > this.max) return this.flush()
    if (!this.cancel) this.cancel = this.later(() => {
      this.cancel = null
      this.flush()
    }, this.flushMs)
  }

  flush(done?: () => void): void {
    this.stopTimer()
    if (!this.size) {
      done?.()
      return
    }
    const bytes = this.chunks.length === 1 ? this.chunks[0]! : concat(this.chunks, this.size)
    this.chunks = []
    this.size = 0
    this.opts.write(bytes, done)
  }

  clear(): void {
    this.stopTimer()
    this.chunks = []
    this.size = 0
    this.carry = ''
  }

  dispose(): void {
    this.clear()
  }

  private keepTail(bytes: Uint8Array): void {
    const tail = bytes.length > CARRY ? bytes.subarray(-CARRY) : bytes
    if (!this.carry && !tail.includes(ESC)) return
    this.setCarry(this.carry + latin1.decode(tail))
  }

  private wantsReply(bytes: Uint8Array): boolean {
    if (!this.carry && !bytes.includes(ESC)) return false
    const text = this.carry + latin1.decode(bytes)
    this.setCarry(text)
    return REPLY_WANTED.test(text)
  }

  private setCarry(text: string): void {
    let end = 0
    for (const m of text.matchAll(REPLY_ALL)) end = m.index + m[0].length
    const rest = text.slice(end)
    const tail = rest.length > CARRY ? rest.slice(-CARRY) : rest
    const at = tail.lastIndexOf('\x1b')
    this.carry = at >= 0 ? tail.slice(at) : ''
  }

  private stopTimer(): void {
    this.cancel?.()
    this.cancel = null
  }
}

function concat(chunks: readonly Uint8Array[], size: number): Uint8Array {
  const out = new Uint8Array(size)
  let at = 0
  for (const c of chunks) {
    out.set(c, at)
    at += c.length
  }
  return out
}
