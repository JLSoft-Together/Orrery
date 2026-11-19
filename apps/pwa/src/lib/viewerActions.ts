import { viewerCall, type Commander } from './viewerApi'

export type ActionResult = { kind: 'done'; data: Record<string, unknown> } | { kind: 'pending' } | { kind: 'cancelled' }

export const SLOW_MS = 60_000

async function act(command: Commander, type: string, fields: Record<string, unknown>, timeoutMs?: number): Promise<ActionResult> {
  const reply = await viewerCall(command, type, fields as never, timeoutMs)
  if (reply.kind !== 'data') return reply
  const data = reply.data
  return { kind: 'done', data: data && typeof data === 'object' && !Array.isArray(data) ? (data as Record<string, unknown>) : {} }
}

export type SaveResult = { kind: 'saved'; mtimeMs: number } | { kind: 'pending' } | { kind: 'cancelled' }

export async function saveFile(command: Commander, path: string, text: string, base: number): Promise<SaveResult> {
  const result = await act(command, 'fs.write', { path, text, base })
  if (result.kind !== 'done') return result
  const mtime = result.data.mtimeMs
  return { kind: 'saved', mtimeMs: typeof mtime === 'number' ? mtime : base }
}

export const isDirty = (original: string, draft: string): boolean => original !== draft

export const stagePaths = (command: Commander, root: string, paths: string[]): Promise<ActionResult> => act(command, 'git.stage', { root, paths })

export const unstagePaths = (command: Commander, root: string, paths: string[]): Promise<ActionResult> => act(command, 'git.unstage', { root, paths })

export const discardPath = (command: Commander, root: string, path: string): Promise<ActionResult> => act(command, 'git.discard', { root, path })

export function commitMessage(raw: string): string | null {
  const text = raw.trim()
  return text === '' ? null : text
}

export function commitStaged(command: Commander, root: string, raw: string): Promise<ActionResult> {
  const message = commitMessage(raw)
  if (!message) return Promise.reject(new Error('empty'))
  return act(command, 'git.commit', { root, message })
}

export const startRun = (command: Commander, root: string, configId: string, deviceId: string | null): Promise<ActionResult> => act(command, 'run.start', { root, configId, deviceId }, SLOW_MS)

export const stopRun = (command: Commander, sessionId: string): Promise<ActionResult> => act(command, 'run.stop', { sessionId })

export const reloadRun = (command: Commander, sessionId: string, hot: boolean): Promise<ActionResult> => act(command, 'run.reload', { sessionId, hot })
