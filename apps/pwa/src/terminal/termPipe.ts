export interface PipeLine {
  translateToString(trimRight?: boolean): string
}

export interface PipeBuffer {
  readonly type: string
  readonly length: number
  getLine(y: number): PipeLine | undefined
  readonly baseY: number
  readonly cursorX: number
  readonly cursorY: number
}

export interface PipeTerm {
  readonly cols: number
  readonly rows: number
  readonly buffer: { readonly active: PipeBuffer }
  write(data: string | Uint8Array, callback?: () => void): void
  resize(cols: number, rows: number): void
}

export interface PipeClock {
  schedule(fn: () => void): number
  cancel(id: number): void
}

const FRAME_CLOCK: PipeClock = {
  schedule: (fn) => requestAnimationFrame(fn),
  cancel: (id) => cancelAnimationFrame(id),
}

export function bufferLines(t: PipeTerm): string[] {
  const b = t.buffer.active
  const out: string[] = []
  for (let i = 0; i < b.length; i++) out.push(b.getLine(i)?.translateToString(true) ?? '')
  return out
}

export function resizeAnchored(t: PipeTerm, cols: number, rows: number): void {
  if (t.cols === cols && t.rows === rows) return
  const before = t.buffer.active
  const normal = before.type === 'normal'
  const baseY = before.baseY
  const oldCols = t.cols
  const oldRows = t.rows
  t.resize(cols, rows)
  if (!normal || rows <= oldRows || cols !== oldCols) return
  const after = t.buffer.active
  const pulled = baseY - after.baseY
  if (pulled <= 0) return
  const y = Math.max(0, after.cursorY - pulled)
  t.write(`\x1b[${rows};1H${'\n'.repeat(pulled)}\x1b[${y + 1};${after.cursorX + 1}H`)
}

export class TermPipe {
  private queue: Uint8Array[] = []
  private held = 0
  private frame = 0
  private readonly term: () => PipeTerm | null
  private readonly clock: PipeClock

  constructor(term: () => PipeTerm | null, clock: PipeClock = FRAME_CLOCK) {
    this.term = term
    this.clock = clock
  }

  get waiting(): boolean {
    return this.held > 0 || this.queue.length > 0
  }

  output(bytes: Uint8Array): void {
    this.queue.push(bytes)
    this.kick()
  }

  private kick(): void {
    if (this.frame || this.held > 0 || this.queue.length === 0) return
    this.frame = this.clock.schedule(() => {
      this.frame = 0
      this.flush()
    })
  }

  flush(): void {
    const t = this.term()
    if (!t || this.held > 0 || this.queue.length === 0) return
    if (this.frame) {
      this.clock.cancel(this.frame)
      this.frame = 0
    }
    const total = this.queue.reduce((n, c) => n + c.length, 0)
    const merged = new Uint8Array(total)
    let at = 0
    for (const chunk of this.queue) {
      merged.set(chunk, at)
      at += chunk.length
    }
    this.queue = []
    t.write(merged)
  }

  barrier(fn: () => void): void {
    this.flush()
    const t = this.term()
    if (!t) {
      fn()
      return
    }
    this.held++
    t.write('', () => {
      try {
        fn()
      } finally {
        this.held--
        this.kick()
      }
    })
  }

  reset(fn: () => void): void {
    this.queue = []
    if (this.frame) {
      this.clock.cancel(this.frame)
      this.frame = 0
    }
    this.term()?.write('\x1bc')
    this.barrier(fn)
  }

  snapshot(cols: number, rows: number, data: string, fit: (cols: number, rows: number) => void): void {
    this.reset(() => {
      fit(cols, rows)
      this.term()?.write(data)
    })
  }

  dispose(): void {
    if (this.frame) this.clock.cancel(this.frame)
    this.frame = 0
    this.queue = []
  }
}
