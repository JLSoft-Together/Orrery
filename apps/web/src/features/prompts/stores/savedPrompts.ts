import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { getPrompts, hasPromptLibrary, onPromptsChanged, setPrompts, type PromptOp, type PromptReply } from '../lib/promptBridge'
import { markMigrated, readLegacy } from '../lib/legacyPrompts'
import { fitsDraft, MAX_ITEMS, type PromptDraft, type PromptError, type SavedPrompt } from '../lib/promptModel'

export type SaveResult = { ok: true; id?: string; items: SavedPrompt[] } | { ok: false; error: PromptError }

export const useSavedPromptsStore = defineStore('savedPrompts', () => {
  const items = ref<SavedPrompt[]>([])
  const loaded = ref(false)
  const error = ref<PromptError | null>(null)
  const recovered = ref(false)
  const supported = hasPromptLibrary
  let sent = 0
  let applied = 0
  let unsub: (() => void) | null = null

  const full = computed(() => items.value.length >= MAX_ITEMS)

  function apply(reply: PromptReply, seq: number): void {
    if (seq < applied) return
    applied = seq
    if (!reply.ok) {
      error.value = reply.error
      return
    }
    error.value = null
    items.value = reply.items
    loaded.value = true
    if (reply.recovered) recovered.value = true
  }

  async function migrate(): Promise<void> {
    const have = new Set(items.value.map((p) => `${p.title}\n${p.body}`))
    const legacy = readLegacy()
    if (!legacy.length) return
    const rows = legacy.filter((r) => !have.has(`${r.title}\n${r.body}`)).slice(0, Math.max(0, MAX_ITEMS - items.value.length))
    if (!rows.length) return markMigrated()
    const seq = ++sent
    const reply = await setPrompts({ op: 'import', items: rows })
    if (reply.ok) {
      markMigrated()
      apply(reply, seq)
    }
  }

  async function load(): Promise<void> {
    const seq = ++sent
    const reply = await getPrompts()
    apply(reply, seq)
    if (reply.ok && seq === sent) await migrate()
  }

  function start(): void {
    if (!supported || unsub) return
    unsub = onPromptsChanged(() => void load())
    void load()
  }

  function stop(): void {
    unsub?.()
    unsub = null
  }

  async function run(op: PromptOp): Promise<SaveResult> {
    const seq = ++sent
    const reply = await setPrompts(op)
    apply(reply, seq)
    return reply.ok ? { ok: true, ...(reply.id ? { id: reply.id } : {}), items: reply.items } : { ok: false, error: reply.error }
  }

  async function save(draft: PromptDraft): Promise<SaveResult> {
    const bad = fitsDraft(draft)
    if (bad) return { ok: false, error: bad }
    return run({ op: 'upsert', item: { ...draft, title: draft.title.trim() } })
  }

  const remove = (id: string): Promise<SaveResult> => run({ op: 'delete', id })

  const reorder = (ids: string[]): Promise<SaveResult> => run({ op: 'reorder', ids })

  async function importRows(rows: { title: string; body: string; scope: string }[]): Promise<SaveResult & { added?: number; skipped?: number }> {
    const seq = ++sent
    const reply = await setPrompts({ op: 'import', items: rows })
    apply(reply, seq)
    if (!reply.ok) return { ok: false, error: reply.error }
    return { ok: true, items: reply.items, added: reply.added ?? 0, skipped: reply.skipped ?? 0 }
  }

  return { items, loaded, error, recovered, supported, full, load, start, stop, save, remove, reorder, importRows }
})
