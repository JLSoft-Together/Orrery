export class SocketClosed extends Error {
  readonly code: number | null
  constructor(code: number | null) {
    super(`closed ${code ?? 'unknown'}`)
    this.code = code
  }
}

export class WireTimeout extends Error {
  constructor(what: string) {
    super(`timeout ${what}`)
  }
}

export interface WireSocket {
  next(timeoutMs?: number): Promise<Uint8Array>
  sendBinary(data: Uint8Array): void
  sendText(text: string): void
  close(code?: number): void
  readonly closeCode: number | null
  readonly lastActivity: number
}

export type WireConnector = (url: string, timeoutMs: number) => Promise<WireSocket>

interface Waiter {
  resolve(data: Uint8Array): void
  reject(error: Error): void
  timer: ReturnType<typeof setTimeout> | null
}

export class BrowserWireSocket implements WireSocket {
  private readonly ws: WebSocket
  private readonly queue: Uint8Array[] = []
  private readonly waiters: Waiter[] = []
  private closed = false
  closeCode: number | null = null
  lastActivity = Date.now()

  constructor(ws: WebSocket) {
    this.ws = ws
    ws.binaryType = 'arraybuffer'
    ws.addEventListener('message', (event) => {
      this.lastActivity = Date.now()
      if (typeof event.data === 'string') return
      const bytes = new Uint8Array(event.data as ArrayBuffer)
      const waiter = this.waiters.shift()
      if (waiter) {
        if (waiter.timer) clearTimeout(waiter.timer)
        waiter.resolve(bytes)
      } else {
        this.queue.push(bytes)
      }
    })
    ws.addEventListener('close', (event) => this.finish(event.code))
    ws.addEventListener('error', () => {})
  }

  private finish(code: number | null): void {
    if (this.closed) return
    this.closed = true
    this.closeCode = code
    for (const waiter of this.waiters.splice(0)) {
      if (waiter.timer) clearTimeout(waiter.timer)
      waiter.reject(new SocketClosed(code))
    }
  }

  next(timeoutMs?: number): Promise<Uint8Array> {
    const ready = this.queue.shift()
    if (ready) return Promise.resolve(ready)
    if (this.closed) return Promise.reject(new SocketClosed(this.closeCode))
    return new Promise<Uint8Array>((resolve, reject) => {
      const waiter: Waiter = { resolve, reject, timer: null }
      if (timeoutMs !== undefined) {
        waiter.timer = setTimeout(() => {
          const at = this.waiters.indexOf(waiter)
          if (at >= 0) this.waiters.splice(at, 1)
          reject(new WireTimeout('read'))
        }, timeoutMs)
      }
      this.waiters.push(waiter)
    })
  }

  sendBinary(data: Uint8Array): void {
    if (this.ws.readyState === WebSocket.OPEN) this.ws.send(data as Uint8Array<ArrayBuffer>)
  }

  sendText(text: string): void {
    if (this.ws.readyState === WebSocket.OPEN) this.ws.send(text)
  }

  close(code?: number): void {
    try {
      if (this.ws.readyState === WebSocket.OPEN || this.ws.readyState === WebSocket.CONNECTING) {
        if (code === undefined) this.ws.close()
        else this.ws.close(code === 1000 || (code >= 3000 && code <= 4999) ? code : 1000)
      }
    } catch {}
    this.finish(this.closeCode ?? code ?? null)
  }
}

export const connectWebSocket: WireConnector = (url, timeoutMs) =>
  new Promise<WireSocket>((resolve, reject) => {
    let ws: WebSocket
    try {
      ws = new WebSocket(url)
    } catch (error) {
      reject(error)
      return
    }
    const wire = new BrowserWireSocket(ws)
    const timer = setTimeout(() => {
      wire.close()
      reject(new WireTimeout('connect'))
    }, timeoutMs)
    ws.addEventListener('open', () => {
      clearTimeout(timer)
      resolve(wire)
    })
    ws.addEventListener('close', (event) => {
      clearTimeout(timer)
      reject(new SocketClosed(event.code))
    })
  })
