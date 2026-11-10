import { renameSync, rmSync, writeFileSync } from 'node:fs'
import path from 'node:path'

export const WEBSTORAGE_EXPORT_FILE = 'webstorage-export.json'

export const DUMP_ALL_LOCAL_JS =
  "(() => { const o = {}; for (let i = 0; i < localStorage.length; i++) { const k = localStorage.key(i); if (k !== null) o[k] = localStorage.getItem(k) } return o })()"

export function exportText(origin: string, local: unknown, savedAt: Date): string | null {
  if (!local || typeof local !== 'object' || Array.isArray(local)) return null
  const entries = Object.entries(local as Record<string, unknown>).filter((e): e is [string, string] => typeof e[1] === 'string')
  return JSON.stringify({ version: 1, origin, savedAt: savedAt.toISOString(), local: Object.fromEntries(entries) })
}

export async function exportWebStorage(dump: () => Promise<unknown>, dataDir: string, origin: string, timeoutMs = 1_500): Promise<boolean> {
  const local = await Promise.race([dump().catch(() => null), new Promise<null>((resolve) => setTimeout(() => resolve(null), timeoutMs))])
  const text = exportText(origin, local, new Date())
  if (!text) return false
  const file = path.join(dataDir, WEBSTORAGE_EXPORT_FILE)
  const tmp = `${file}.${process.pid}.tmp`
  try {
    writeFileSync(tmp, text)
    renameSync(tmp, file)
    return true
  } catch {
    rmSync(tmp, { force: true })
    return false
  }
}
