import type { JsonMap } from '../remote/types'
import { viewerData, type Commander } from './viewerApi'
import { parseFile, samePath } from './viewerModel'

export const OWN_WRITE_MS = 3000

export function touchesPath(event: JsonMap, path: string): boolean {
  if (event.t !== 'fs.changed' || !Array.isArray(event.paths)) return false
  return event.paths.some((p) => typeof p === 'string' && samePath(p, path))
}

export function isOwnChange(saving: boolean, lastSaveAt: number | null, now: number): boolean {
  return saving || (lastSaveAt !== null && now - lastSaveAt < OWN_WRITE_MS)
}

export function watchRoot(command: Commander, root: string): void {
  void command('fs.watch', { root }).catch(() => null)
}

export function unwatchRoot(command: Commander, root: string): void {
  void command('fs.unwatch', { root }).catch(() => null)
}

export async function currentStamp(command: Commander, path: string): Promise<number> {
  return parseFile(await viewerData(command, 'fs.read', { path })).mtimeMs
}
