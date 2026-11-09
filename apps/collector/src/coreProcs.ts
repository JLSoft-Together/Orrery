import { spawn, type ChildProcess } from 'node:child_process'

export interface CoreProcRow {
  pid: number
  ppid: number
  name: string
  startMs: number | null
  startRaw: string | null
}

export interface CoreUsageRow {
  pid: number
  workingSet: number | null
  privateBytes: number | null
  cpuMs: number | null
}

export interface CoreCmdRow {
  pid: number
  cmd: string
}

export interface CorePortRow {
  pid: number
  state: number
  address: string
  port: number
}

export interface CoreProcsClient {
  ping(): Promise<string | null>
  table(): Promise<CoreProcRow[] | null>
  usage(pids: readonly number[]): Promise<CoreUsageRow[] | null>
  cmdline(pids: readonly number[]): Promise<CoreCmdRow[] | null>
  ports(): Promise<CorePortRow[] | null>
  dispose(): void
}

export interface CoreProcsOptions {
  bin: string
  args?: string[]
  timeoutMs?: number
  maxRestarts?: number
  windowMs?: number
  now?: () => number
  log?: (line: string) => void
}

const MAX_PIDS = 4096
const MAX_LINE = 2 * 1024 * 1024

type Reply = Record<string, unknown>

interface Pending {
  resolve: (reply: Reply | null) => void
  timer: NodeJS.Timeout
}

const isObj = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null
const int = (v: unknown): number | null => (typeof v === 'number' && Number.isInteger(v) && v >= 0 ? v : null)
const numOrNull = (v: unknown): number | null => (typeof v === 'number' && Number.isFinite(v) && v >= 0 ? v : null)

function rowsOf(reply: Reply | null): Record<string, unknown>[] | null {
  if (!reply || !Array.isArray(reply.rows)) return null
  return reply.rows.filter(isObj)
}

export function parseTableRows(reply: Reply | null): CoreProcRow[] | null {
  const rows = rowsOf(reply)
  if (!rows) return null
  const out: CoreProcRow[] = []
  for (const r of rows) {
    const pid = int(r.pid)
    const ppid = int(r.ppid)
    if (pid === null || ppid === null) continue
    out.push({
      pid,
      ppid,
      name: typeof r.name === 'string' ? r.name : '',
      startMs: numOrNull(r.startMs),
      startRaw: typeof r.startRaw === 'string' && r.startRaw !== '' ? r.startRaw : null,
    })
  }
  return out
}

export function parseUsageRows(reply: Reply | null): CoreUsageRow[] | null {
  const rows = rowsOf(reply)
  if (!rows) return null
  const out: CoreUsageRow[] = []
  for (const r of rows) {
    const pid = int(r.pid)
    if (pid === null) continue
    out.push({ pid, workingSet: numOrNull(r.workingSet), privateBytes: numOrNull(r.privateBytes), cpuMs: numOrNull(r.cpuMs) })
  }
  return out
}

export function parseCmdRows(reply: Reply | null): CoreCmdRow[] | null {
  const rows = rowsOf(reply)
  if (!rows) return null
  const out: CoreCmdRow[] = []
  for (const r of rows) {
    const pid = int(r.pid)
    if (pid !== null && typeof r.cmd === 'string') out.push({ pid, cmd: r.cmd })
  }
  return out
}

export function parsePortRows(reply: Reply | null): CorePortRow[] | null {
  const rows = rowsOf(reply)
  if (!rows) return null
  const out: CorePortRow[] = []
  for (const r of rows) {
    const pid = int(r.pid)
    const state = int(r.state)
    const port = int(r.port)
    if (pid === null || state === null || port === null || port === 0 || port > 65_535) continue
    out.push({ pid, state, address: typeof r.address === 'string' ? r.address : '', port })
  }
  return out
}

export function createCoreProcs(opts: CoreProcsOptions): CoreProcsClient {
  const args = opts.args ?? ['procs']
  const timeoutMs = opts.timeoutMs ?? 5_000
  const PORTS_TIMEOUT_MS = 20_000
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
    const id = int(msg.id)
    if (id === null) return
    const p = pending.get(id)
    if (!p) return
    pending.delete(id)
    clearTimeout(p.timer)
    p.resolve(msg.ok === true ? msg : null)
  }

  const reader = (owner: () => ChildProcess | null, mine: () => ChildProcess): ((chunk: string) => void) => {
    let buffer = ''
    return (chunk) => {
      if (owner() !== mine()) return
      buffer += chunk
      let at = buffer.indexOf('\n')
      while (at >= 0) {
        onLine(buffer.slice(0, at))
        buffer = buffer.slice(at + 1)
        at = buffer.indexOf('\n')
      }
      if (buffer.length > MAX_LINE) buffer = ''
    }
  }

  const drop = (dead: ChildProcess, why: string): void => {
    if (child !== dead) return
    child = null
    log(`[ccm] core-procs child ended (${why}), falling back until restarted`)
    failAll()
  }

  const start = (): ChildProcess | null => {
    if (disposed) return null
    if (child) return child
    const t = now()
    while (spawns.length && t - spawns[0]! >= windowMs) spawns.shift()
    if (spawns.length > maxRestarts) {
      if (!gaveUp) {
        gaveUp = true
        log(`[ccm] core-procs restarted more than ${maxRestarts} times in ${Math.round(windowMs / 1000)} s, using the legacy process reader`)
      }
      return null
    }
    gaveUp = false
    spawns.push(t)
    let c: ChildProcess
    try {
      c = spawn(opts.bin, args, { stdio: ['pipe', 'pipe', 'ignore'], windowsHide: true })
    } catch (err) {
      log(`[ccm] core-procs spawn failed: ${(err as Error).message}`)
      return null
    }
    c.stdout?.setEncoding('utf8')
    c.stdout?.on('data', reader(() => child, () => c))
    c.stdout?.on('error', () => undefined)
    c.stdin?.on('error', () => undefined)
    c.on('error', (err) => drop(c, err.message))
    c.on('exit', (code, signal) => drop(c, `exit ${code ?? signal}`))
    c.unref()
    ;(c.stdout as { unref?: () => void } | null)?.unref?.()
    ;(c.stdin as { unref?: () => void } | null)?.unref?.()
    child = c
    return c
  }

  const request = (op: string, extra?: Record<string, unknown>, waitMs = timeoutMs): Promise<Reply | null> => {
    const c = start()
    if (!c?.stdin || c.stdin.destroyed || !c.stdin.writable) return Promise.resolve(null)
    const id = nextId++
    return new Promise((resolve) => {
      const timer = setTimeout(() => {
        pending.delete(id)
        resolve(null)
        log(`[ccm] core-procs ${op} timed out after ${waitMs} ms, restarting the child`)
        const dead = child
        if (dead) {
          drop(dead, 'timeout')
          dead.kill()
        }
      }, waitMs)
      pending.set(id, { resolve, timer })
      c.stdin!.write(`${JSON.stringify({ id, op, ...extra })}\n`, (err) => {
        if (!err) return
        const p = pending.get(id)
        if (!p) return
        pending.delete(id)
        clearTimeout(p.timer)
        p.resolve(null)
      })
    })
  }

  const chunked = async <T>(pids: readonly number[], op: string, parse: (reply: Reply | null) => T[] | null): Promise<T[] | null> => {
    const valid = pids.filter((p) => Number.isInteger(p) && p > 0)
    if (valid.length === 0) return []
    const out: T[] = []
    for (let i = 0; i < valid.length; i += MAX_PIDS) {
      const rows = parse(await request(op, { pids: valid.slice(i, i + MAX_PIDS) }))
      if (!rows) return null
      out.push(...rows)
    }
    return out
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
      } catch {
        c.kill()
      }
      c.kill()
    }
  }

  process.on('exit', onExit)

  return {
    async ping() {
      const reply = await request('ping')
      return reply && typeof reply.version === 'string' ? reply.version : null
    },
    async table() {
      return parseTableRows(await request('table'))
    },
    usage: (pids) => chunked(pids, 'usage', parseUsageRows),
    cmdline: (pids) => chunked(pids, 'cmdline', parseCmdRows),
    async ports() {
      return parsePortRows(await request('ports', undefined, Math.max(timeoutMs, PORTS_TIMEOUT_MS)))
    },
    dispose,
  }
}
