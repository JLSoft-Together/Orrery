import { promises as fs } from 'node:fs'
import path from 'node:path'
import type { DiskHistoryEntry } from '@ccm/shared'

export const HISTORY_FILE = 'history.jsonl'
const MAX_PATHS = 50
const MAX_BYTES = 1024 * 1024
const KEEP_LINES = 1000

export function cleanHistoryEntry(v: unknown): DiskHistoryEntry | null {
  if (!v || typeof v !== 'object' || Array.isArray(v)) return null
  const o = v as Record<string, unknown>
  const n = (x: unknown): number | null => (typeof x === 'number' && Number.isFinite(x) && x >= 0 ? x : null)
  const at = n(o.at)
  const count = n(o.count)
  const freed = n(o.freed)
  const failed = n(o.failed)
  if (at === null || count === null || freed === null || failed === null) return null
  if (o.action !== 'trash' && o.action !== 'delete' && o.action !== 'move') return null
  const paths = Array.isArray(o.paths) ? o.paths.filter((p): p is string => typeof p === 'string').slice(0, MAX_PATHS) : []
  return { at, action: o.action, count, freed, paths, failed }
}

export class DiskHistory {
  private queue: Promise<void> = Promise.resolve()

  constructor(private readonly dir: string) {}

  get file(): string {
    return path.join(this.dir, HISTORY_FILE)
  }

  append(entry: DiskHistoryEntry): Promise<void> {
    const clean = cleanHistoryEntry({ ...entry, paths: entry.paths.slice(0, MAX_PATHS) })
    if (!clean) return Promise.resolve()
    const run = async (): Promise<void> => {
      await fs.mkdir(this.dir, { recursive: true })
      await fs.appendFile(this.file, `${JSON.stringify(clean)}\n`, 'utf8')
      const st = await fs.stat(this.file)
      if (st.size > MAX_BYTES) {
        const lines = (await fs.readFile(this.file, 'utf8')).split('\n').filter((l) => l.trim())
        const tmp = `${this.file}.tmp`
        await fs.writeFile(tmp, `${lines.slice(-KEEP_LINES).join('\n')}\n`, 'utf8')
        await fs.rename(tmp, this.file)
      }
    }
    this.queue = this.queue.then(run, run)
    return this.queue
  }

  async read(limit: number): Promise<DiskHistoryEntry[]> {
    await this.queue.catch(() => undefined)
    let text: string
    try {
      text = await fs.readFile(this.file, 'utf8')
    } catch {
      return []
    }
    const out: DiskHistoryEntry[] = []
    const lines = text.split('\n')
    for (let i = lines.length - 1; i >= 0 && out.length < limit; i--) {
      const line = lines[i]!.trim()
      if (!line) continue
      try {
        const e = cleanHistoryEntry(JSON.parse(line))
        if (e) out.push(e)
      } catch {
        continue
      }
    }
    return out
  }
}
