import { desktop } from '../../../core/lib/desktop'
import { isPrompt, toPromptError, type PromptDraft, type PromptError, type SavedPrompt } from './promptModel'

export type PromptReply = { ok: true; items: SavedPrompt[]; id?: string; added?: number; skipped?: number; recovered?: boolean } | { ok: false; error: PromptError }

export type PromptOp =
  | { op: 'upsert'; item: PromptDraft }
  | { op: 'delete'; id: string }
  | { op: 'reorder'; ids: string[] }
  | { op: 'import'; items: { title: string; body: string; scope: string }[] }

export const hasPromptLibrary = typeof desktop?.promptsGet === 'function' && typeof desktop?.promptsSet === 'function'

function parseReply(raw: unknown): PromptReply {
  if (!raw || typeof raw !== 'object') return { ok: false, error: 'io' }
  const r = raw as Record<string, unknown>
  if (r.ok !== true) return { ok: false, error: toPromptError(r.error) }
  const items = Array.isArray(r.items) ? r.items.filter(isPrompt) : []
  return {
    ok: true,
    items,
    ...(typeof r.id === 'string' ? { id: r.id } : {}),
    ...(typeof r.added === 'number' ? { added: r.added } : {}),
    ...(typeof r.skipped === 'number' ? { skipped: r.skipped } : {}),
    ...(r.recovered === true ? { recovered: true } : {}),
  }
}

export async function getPrompts(): Promise<PromptReply> {
  if (!hasPromptLibrary) return { ok: false, error: 'unavailable' }
  try {
    return parseReply(await desktop!.promptsGet!())
  } catch {
    return { ok: false, error: 'io' }
  }
}

export async function setPrompts(op: PromptOp): Promise<PromptReply> {
  if (!hasPromptLibrary) return { ok: false, error: 'unavailable' }
  try {
    return parseReply(await desktop!.promptsSet!(op))
  } catch {
    return { ok: false, error: 'io' }
  }
}

interface FileBridge {
  promptsExport?: (text: string) => Promise<unknown>
  promptsImport?: () => Promise<unknown>
}

const fileBridge = desktop as unknown as FileBridge | undefined

export const hasPromptFiles = typeof fileBridge?.promptsImport === 'function'

export type FileResult<T> = { ok: true; value: T } | { ok: false; error: string }

const fileError = (raw: unknown): string => (raw && typeof raw === 'object' && typeof (raw as { error?: unknown }).error === 'string' ? (raw as { error: string }).error : 'failed')

export async function exportPromptsFile(text: string): Promise<FileResult<{ count: number }>> {
  try {
    const raw = await fileBridge?.promptsExport?.(text)
    const r = raw as { ok?: unknown; count?: unknown } | null
    return r?.ok === true ? { ok: true, value: { count: typeof r.count === 'number' ? r.count : 0 } } : { ok: false, error: fileError(raw) }
  } catch {
    return { ok: false, error: 'failed' }
  }
}

export async function pickPromptsFile(): Promise<FileResult<unknown>> {
  try {
    const raw = await fileBridge?.promptsImport?.()
    const r = raw as { ok?: unknown; data?: unknown } | null
    return r?.ok === true ? { ok: true, value: r.data } : { ok: false, error: fileError(raw) }
  } catch {
    return { ok: false, error: 'failed' }
  }
}

export const onPromptsChanged = (cb: () => void): (() => void) => desktop?.onPromptsChanged?.(() => cb()) ?? (() => {})
