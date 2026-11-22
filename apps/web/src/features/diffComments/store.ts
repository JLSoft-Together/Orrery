import { ref } from 'vue'
import { defineStore } from 'pinia'
import type { DiffFile } from '../../lib/diff'
import { dirKey } from '../../stores/favorites'
import { isTerm, presetOf, type Pane } from '../../stores/workspace'
import { inRoot } from '../../lib/runUi'
import { addComment, normRange, parseComments, reconcile, snippetOf, type CommentDraft, type CommentSide, type DiffComment } from './diffComments'

const STORAGE = 'ccm.diffComments.'
const EMPTY: readonly DiffComment[] = Object.freeze([])

export function agentTargets(panes: readonly Pane[], root: string): Pane[] {
  return panes
    .filter((p) => isTerm(p) && (presetOf(p) === 'claude' || presetOf(p) === 'codex') && p.state === 'running' && !!p.cwd && inRoot(p.cwd, root))
    .sort((a, b) => (b.focusedAt ?? 0) - (a.focusedAt ?? 0))
}

export const useDiffCommentsStore = defineStore('diffComments', () => {
  const byRepo = ref<Record<string, DiffComment[]>>({})
  const draft = ref<CommentDraft | null>(null)
  let seq = 0

  function load(key: string): DiffComment[] {
    let raw: string | null = null
    try {
      raw = localStorage.getItem(STORAGE + key)
    } catch {
      raw = null
    }
    return parseComments(raw)
  }

  function save(key: string, list: readonly DiffComment[]): void {
    try {
      if (list.length) localStorage.setItem(STORAGE + key, JSON.stringify(list))
      else localStorage.removeItem(STORAGE + key)
    } catch {
      return
    }
  }

  function ensure(root: string): void {
    const key = dirKey(root)
    if (!(key in byRepo.value)) byRepo.value = { ...byRepo.value, [key]: load(key) }
  }

  function list(root: string | null | undefined): readonly DiffComment[] {
    return root ? (byRepo.value[dirKey(root)] ?? EMPTY) : EMPTY
  }

  function set(root: string, next: DiffComment[]): void {
    const key = dirKey(root)
    byRepo.value = { ...byRepo.value, [key]: next }
    save(key, next)
  }

  function add(root: string, input: { path: string; side: CommentSide; from: number; to: number; code: string; text: string }): DiffComment | null {
    const text = input.text.trim()
    if (!text) return null
    ensure(root)
    const [from, to] = normRange(input.from, input.to)
    const c: DiffComment = { id: `c${Date.now().toString(36)}${(++seq).toString(36)}`, path: input.path, side: input.side, from, to, code: input.code, text, stale: false, at: Date.now() }
    set(root, addComment(list(root), c))
    return c
  }

  function edit(root: string, id: string, text: string): void {
    const body = text.trim()
    if (!body) return remove(root, id)
    set(
      root,
      list(root).map((c) => (c.id === id ? { ...c, text: body } : c)),
    )
  }

  function remove(root: string, id: string): void {
    set(
      root,
      list(root).filter((c) => c.id !== id),
    )
    if (draft.value?.editId === id) draft.value = null
  }

  function clear(root: string): void {
    set(root, [])
    if (draft.value && dirKey(draft.value.root) === dirKey(root)) draft.value = null
  }

  function reconcileFile(root: string, path: string, file: DiffFile): void {
    ensure(root)
    const next = reconcile(list(root), path, file)
    if (next) set(root, next)
  }

  function openDraft(next: CommentDraft): void {
    draft.value = next
  }

  function extendDraft(line: number): boolean {
    const d = draft.value
    if (!d || d.editId) return false
    const [from, to] = normRange(Math.min(d.from, line), Math.max(d.to, line))
    draft.value = { ...d, from, to }
    return true
  }

  function closeDraft(): void {
    draft.value = null
  }

  function commitDraft(text: string, file: DiffFile | null): void {
    const d = draft.value
    draft.value = null
    if (!d) return
    if (d.editId) return edit(d.root, d.editId, text)
    add(d.root, { path: d.path, side: d.side, from: d.from, to: d.to, code: file ? snippetOf(file, d.side, d.from, d.to) : '', text })
  }

  return { byRepo, draft, ensure, list, add, edit, remove, clear, reconcileFile, openDraft, extendDraft, closeDraft, commitDraft }
})
