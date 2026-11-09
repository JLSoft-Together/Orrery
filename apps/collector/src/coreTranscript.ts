import { spawn, type ChildProcess } from 'node:child_process'

export type CoreMessage = Record<string, unknown>

export interface CoreRequestOptions {
  timeoutMs?: number
  restartOnTimeout?: boolean
}

export interface CoreTranscriptClient {
  request(op: string, extra?: Record<string, unknown>, opts?: CoreRequestOptions): Promise<CoreMessage | null>
  send(op: string, extra?: Record<string, unknown>): boolean
  dispose(): void
}

export interface CoreTranscriptOptions {
  bin: string
  args?: string[]
  timeoutMs?: number
  maxRestarts?: number
  windowMs?: number
  now?: () => number
  log?: (line: string) => void
  onEvent?: (event: CoreMessage) => void
  onDown?: () => void
}

const MAX_LINE = 256 * 1024 * 1024
const LOG_LINE_MAX = 500

interface Pending {
  resolve: (reply: CoreMessage | null) => void
  timer: NodeJS.Timeout
}

const isObj = (v: unknown): v is CoreMessage => typeof v === 'object' && v !== null && !Array.isArray(v)

function lineReader(onLine: (line: string) => void, live: () => boolean): (chunk: string) => void {
  let parts: string[] = []
  let size = 0
  return (chunk) => {
    if (!live()) return
    let start = 0
    let at = chunk.indexOf('\n')
    while (at >= 0) {
      parts.push(chunk.slice(start, at))
      const line = parts.join('')
      parts = []
      size = 0
      onLine(line)
      start = at + 1
      at = chunk.indexOf('\n', start)
    }
    if (start < chunk.length) {
      parts.push(chunk.slice(start))
      size += chunk.length - start
      if (size > MAX_LINE) {
        parts = []
        size = 0
      }
    }
  }
}

export function createCoreTranscript(opts: CoreTranscriptOptions): CoreTranscriptClient {
  const args = opts.args ?? ['transcript']
  const timeoutMs = opts.timeoutMs ?? 5_000
  const maxRestarts = opts.maxRestarts ?? 3
  const windowMs = opts.windowMs ?? 60_000
  const now = opts.now ?? Date.now
  const log = opts.log ?? ((line: string) => console.warn(line))

  let child: ChildProcess | null = null
  let disposed = false
  let nextId = 1
  let gaveUp = false
  const spawns: number[] = []
  const pending = new Map<number, Pending>()

  const failAll = (): void => {
    for (const p of pending.values()) {
      clearTimeout(p.timer)
      p.resolve(null)
    }
    pending.clear()
  }

  const onLine = (line: string): void => {
    if (!line.trim()) return
    let msg: unknown
    try {
      msg = JSON.parse(line)
    } catch {
      return
    }
    if (!isObj(msg)) return
    if (typeof msg.event === 'string') {
      opts.onEvent?.(msg)
      return
    }
    const id = msg.id
    if (typeof id !== 'number') return
    const p = pending.get(id)
    if (!p) return
    pending.delete(id)
    clearTimeout(p.timer)
    p.resolve(msg.ok === true ? msg : null)
  }

  const drop = (dead: ChildProcess, why: string): void => {
    if (child !== dead) return
    child = null
    log(`[ccm] core-transcript child ended (${why})`)
    failAll()
    if (!disposed) opts.onDown?.()
  }

  const start = (): ChildProcess | null => {
    if (disposed) return null
    if (child) return child
    const t = now()
    while (spawns.length && t - spawns[0]! >= windowMs) spawns.shift()
    if (spawns.length > maxRestarts) {
      if (!gaveUp) {
        gaveUp = true
        log(`[ccm] core-transcript restarted more than ${maxRestarts} times in ${Math.round(windowMs / 1000)} s, giving up`)
      }
      return null
    }
    gaveUp = false
    spawns.push(t)
    let c: ChildProcess
    try {
      c = spawn(opts.bin, args, { stdio: ['pipe', 'pipe', 'pipe'], windowsHide: true })
    } catch (err) {
      log(`[ccm] core-transcript spawn failed: ${(err as Error).message}`)
      return null
    }
    const live = () => child === c
    c.stdout?.setEncoding('utf8')
    c.stdout?.on('data', lineReader(onLine, live))
    c.stdout?.on('error', () => undefined)
    c.stderr?.setEncoding('utf8')
    c.stderr?.on('data', lineReader((l) => l.trim() && console.error(l.slice(0, LOG_LINE_MAX)), () => true))
    c.stderr?.on('error', () => undefined)
    c.stdin?.on('error', () => undefined)
    c.on('error', (err) => drop(c, err.message))
    c.on('exit', (code, signal) => drop(c, `exit ${code ?? signal}`))
    c.unref()
    for (const s of [c.stdout, c.stderr, c.stdin]) (s as { unref?: () => void } | null)?.unref?.()
    child = c
    return c
  }

  const writable = (c: ChildProcess | null): c is ChildProcess => !!c?.stdin && !c.stdin.destroyed && c.stdin.writable

  const request = (op: string, extra?: Record<string, unknown>, ro: CoreRequestOptions = {}): Promise<CoreMessage | null> => {
    const c = start()
    if (!writable(c)) return Promise.resolve(null)
    const id = nextId++
    const waitMs = ro.timeoutMs ?? timeoutMs
    return new Promise((resolve) => {
      const timer = setTimeout(() => {
        pending.delete(id)
        resolve(null)
        if (ro.restartOnTimeout === false) return
        log(`[ccm] core-transcript ${op} timed out after ${waitMs} ms, restarting the child`)
        const dead = child
        if (dead) {
          drop(dead, 'timeout')
          dead.kill()
        }
      }, waitMs)
      pending.set(id, { resolve, timer })
      c.stdin!.write(`${JSON.stringify({ ...extra, id, op })}\n`, (err) => {
        if (!err) return
        const p = pending.get(id)
        if (!p) return
        pending.delete(id)
        clearTimeout(p.timer)
        p.resolve(null)
      })
    })
  }

  const send = (op: string, extra?: Record<string, unknown>): boolean => {
    const c = child
    if (!writable(c)) return false
    c.stdin!.write(`${JSON.stringify({ ...extra, op })}\n`)
    return true
  }

  const onExit = (): void => dispose()

  function dispose(): void {
    if (disposed) return
    disposed = true
    process.off('exit', onExit)
    failAll()
    const c = child
    child = null
    if (c) {
      try {
        c.stdin?.end()
        setTimeout(() => c.exitCode === null && c.kill(), 5_000).unref()
      } catch {
        c.kill()
      }
    }
  }

  process.on('exit', onExit)

  return { request, send, dispose }
}
