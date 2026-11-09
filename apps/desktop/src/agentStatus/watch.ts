import { watch as fsWatch, promises as fs, type FSWatcher } from 'node:fs'
import path from 'node:path'
import { AGENT_STATUS_DIR, AGENT_STATUS_FILE, AGENT_STATUS_MAX_BYTES, parseAgentStatus, type WorkflowStatus } from '@ccm/shared'
import { isWithin } from '../preview/paths'

export type StatusRead = { kind: 'ok'; status: WorkflowStatus } | { kind: 'missing' } | { kind: 'invalid' }

const realOrNull = (p: string): Promise<string | null> => fs.realpath(p).catch(() => null)

/** Real path of `<dir>/.orrery`, null when it is missing or resolves outside `dir` (symlink / junction escape). */
export async function resolveStatusDir(dir: string): Promise<string | null> {
  const root = await realOrNull(dir)
  const orrery = root ? await realOrNull(path.join(root, AGENT_STATUS_DIR)) : null
  return root && orrery && orrery !== root && isWithin(root, orrery) ? orrery : null
}

export async function readStatus(orreryDir: string): Promise<StatusRead> {
  const realDir = (await realOrNull(orreryDir)) ?? orreryDir
  const file = await realOrNull(path.join(orreryDir, AGENT_STATUS_FILE))
  if (!file || !isWithin(realDir, file)) return { kind: 'missing' }
  let handle: fs.FileHandle | null = null
  try {
    handle = await fs.open(file, 'r')
    const st = await handle.stat()
    if (!st.isFile()) return { kind: 'missing' }
    if (st.size > AGENT_STATUS_MAX_BYTES) return { kind: 'invalid' }
    const buf = Buffer.alloc(AGENT_STATUS_MAX_BYTES + 1)
    const { bytesRead } = await handle.read(buf, 0, buf.length, 0)
    if (bytesRead > AGENT_STATUS_MAX_BYTES) return { kind: 'invalid' }
    const status = parseAgentStatus(buf.toString('utf8', 0, bytesRead), Math.round(st.mtimeMs))
    return status ? { kind: 'ok', status } : { kind: 'invalid' }
  } catch {
    return { kind: 'missing' }
  } finally {
    await handle?.close().catch(() => undefined)
  }
}

type Watcher = Pick<FSWatcher, 'close' | 'on'>

export interface StatusWatchOptions {
  debounceMs?: number
  /** Re-check of a folder whose `.orrery` does not exist yet. */
  recheckMs?: number
  /** A status file that stops parsing keeps its last good value this long. */
  holdMs?: number
  watch?: (dir: string, listener: (event: string, file: string | Buffer | null) => void) => Watcher
  resolve?: (dir: string) => Promise<string | null>
  read?: (orreryDir: string) => Promise<StatusRead>
}

const KEYS_MAX = 64
const DIR_MAX = 1024

interface Entry {
  dir: string
  gen: number
  watcher: Watcher | null
  watching: string | null
  debounce: NodeJS.Timeout | null
  recheck: NodeJS.Timeout | null
  hold: NodeJS.Timeout | null
  last: WorkflowStatus | null
}

const same = (a: WorkflowStatus | null, b: WorkflowStatus | null): boolean =>
  a === b || (!!a && !!b && a.phase === b.phase && a.progress === b.progress && a.note === b.note && a.updatedAt === b.updatedAt)

/** One fs.watch per agent pane on `<pane folder>/.orrery` (never polling while the folder exists). */
export class AgentStatusWatch {
  private readonly entries = new Map<string, Entry>()
  private readonly debounceMs: number
  private readonly recheckMs: number
  private readonly holdMs: number

  constructor(
    private readonly onStatus: (key: string, status: WorkflowStatus | null) => void,
    private readonly opts: StatusWatchOptions = {},
  ) {
    this.debounceMs = opts.debounceMs ?? 300
    this.recheckMs = opts.recheckMs ?? 30_000
    this.holdMs = opts.holdMs ?? 5_000
  }

  /** `dir` null stops watching `key`. The same folder again re-sends the current value (page reload). */
  set(key: string, dir: string | null): void {
    if (dir === null || typeof dir !== 'string' || dir.length > DIR_MAX || !path.isAbsolute(dir)) return this.drop(key)
    const known = this.entries.get(key)
    if (known?.dir === dir) {
      this.onStatus(key, known.last)
      return
    }
    this.drop(key)
    if (this.entries.size >= KEYS_MAX) return
    const entry: Entry = { dir, gen: 0, watcher: null, watching: null, debounce: null, recheck: null, hold: null, last: null }
    this.entries.set(key, entry)
    void this.probe(key, entry)
  }

  has(key: string): boolean {
    return this.entries.has(key)
  }

  dispose(): void {
    for (const key of [...this.entries.keys()]) this.drop(key)
  }

  private drop(key: string): void {
    const e = this.entries.get(key)
    if (!e) return
    e.gen++
    this.detach(e)
    for (const t of [e.debounce, e.recheck, e.hold]) if (t) clearTimeout(t)
    this.entries.delete(key)
  }

  private detach(e: Entry): void {
    try {
      e.watcher?.close()
    } catch {
      // already closed
    }
    e.watcher = null
    e.watching = null
  }

  private schedule(key: string, e: Entry, ms: number): void {
    if (e.debounce) clearTimeout(e.debounce)
    const gen = e.gen
    e.debounce = setTimeout(() => {
      e.debounce = null
      if (e.gen === gen) void this.probe(key, e)
    }, ms)
    e.debounce.unref?.()
  }

  private armRecheck(key: string, e: Entry): void {
    if (e.recheck) return
    const gen = e.gen
    e.recheck = setTimeout(() => {
      e.recheck = null
      if (e.gen === gen) void this.probe(key, e)
    }, this.recheckMs)
    e.recheck.unref?.()
  }

  private async probe(key: string, e: Entry): Promise<void> {
    const gen = e.gen
    const orrery = await (this.opts.resolve ?? resolveStatusDir)(e.dir)
    if (e.gen !== gen) return
    if (!orrery) {
      this.detach(e)
      this.apply(key, e, { kind: 'missing' })
      this.armRecheck(key, e)
      return
    }
    if (e.watching !== orrery) this.attach(key, e, orrery)
    const read = await (this.opts.read ?? readStatus)(orrery)
    if (e.gen === gen) this.apply(key, e, read)
  }

  private attach(key: string, e: Entry, orrery: string): void {
    this.detach(e)
    try {
      const make = this.opts.watch ?? ((dir, listener) => fsWatch(dir, { persistent: false }, listener))
      const w = make(orrery, (_event, file) => {
        const name = file === null || file === undefined ? null : String(file)
        if (name === null || name.toLowerCase() === AGENT_STATUS_FILE) this.schedule(key, e, this.debounceMs)
      })
      // Folder removed or watch limit hit: look again now, the slow re-check takes over if it is gone.
      w.on('error', () => {
        this.detach(e)
        this.schedule(key, e, this.debounceMs)
      })
      e.watcher = w
      e.watching = orrery
    } catch {
      this.armRecheck(key, e)
    }
  }

  private apply(key: string, e: Entry, read: StatusRead): void {
    if (read.kind === 'invalid') {
      if (e.last && !e.hold) {
        const gen = e.gen
        e.hold = setTimeout(() => {
          e.hold = null
          if (e.gen === gen) this.publish(key, e, null)
        }, this.holdMs)
        e.hold.unref?.()
      }
      return
    }
    if (e.hold) {
      clearTimeout(e.hold)
      e.hold = null
    }
    this.publish(key, e, read.kind === 'ok' ? read.status : null)
  }

  private publish(key: string, e: Entry, status: WorkflowStatus | null): void {
    if (same(e.last, status)) return
    e.last = status
    this.onStatus(key, status)
  }
}
