import fs from 'node:fs/promises'
import path from 'node:path'
import { Cancelled } from '../agentData/fsTree'

const STAT_BATCH = 128

export async function dirSize(root: string, signal?: AbortSignal): Promise<number> {
  let bytes = 0
  const stack: string[] = [root]
  while (stack.length) {
    if (signal?.aborted) throw new Cancelled()
    const dir = stack.pop()!
    let entries: import('node:fs').Dirent[]
    try {
      entries = await fs.readdir(dir, { withFileTypes: true })
    } catch {
      continue
    }
    const files: string[] = []
    for (const e of entries) {
      if (e.isSymbolicLink()) continue
      const p = path.join(dir, e.name)
      if (e.isDirectory()) stack.push(p)
      else if (e.isFile()) files.push(p)
    }
    for (let i = 0; i < files.length; i += STAT_BATCH) {
      const sizes = await Promise.all(files.slice(i, i + STAT_BATCH).map((f) => fs.lstat(f).then((s) => s.size, () => 0)))
      for (const s of sizes) bytes += s
    }
  }
  return bytes
}
