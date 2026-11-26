import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'

interface Row {
  id: string
  title: string
  body: string
  scope: string
  updatedAt: number
}

const core = vi.hoisted(() => {
  const state = {
    items: [] as Row[],
    calls: [] as unknown[],
    changed: null as null | (() => void),
    off: 0,
    getReply: null as null | (() => Promise<unknown>),
    next: 1,
  }
  return state
})

vi.mock('../../../core/lib/desktop', () => ({
  hasTerminals: true,
  desktop: {
    promptsGet: async () => (core.getReply ? core.getReply() : { ok: true, version: 1, items: core.items }),
    promptsSet: async (req: { op: string; item?: Row; id?: string; ids?: string[]; items?: Row[] }) => {
      core.calls.push(req)
      if (req.op === 'upsert') {
        const it = req.item!
        const at = core.items.findIndex((x) => x.id === it.id)
        const row = { ...it, id: it.id ?? `id${core.next++}`, updatedAt: 1 }
        if (at >= 0) core.items[at] = row
        else core.items.push(row)
        return { ok: true, version: 1, items: core.items, id: row.id }
      }
      if (req.op === 'delete') {
        core.items = core.items.filter((x) => x.id !== req.id)
        return { ok: true, version: 1, items: core.items }
      }
      if (req.op === 'reorder') {
        core.items = req.ids!.map((id) => core.items.find((x) => x.id === id)!)
        return { ok: true, version: 1, items: core.items }
      }
      if (req.op === 'import') {
        for (const r of req.items!) core.items.push({ ...r, id: `id${core.next++}`, updatedAt: 1 })
        return { ok: true, version: 1, items: core.items, added: req.items!.length, skipped: 0 }
      }
      return { ok: false, error: 'invalid-op' }
    },
    onPromptsChanged: (cb: () => void) => {
      core.changed = cb
      return () => void core.off++
    },
  },
}))

const { useSavedPromptsStore } = await import('./savedPrompts')
const { LEGACY_KEY, MIGRATED_KEY } = await import('../lib/legacyPrompts')

const draft = (title: string) => ({ title, body: `${title} body`, scope: 'all' })
const settle = () => new Promise((r) => setTimeout(r, 0))

describe('saved prompts store', () => {
  beforeEach(() => {
    localStorage.clear()
    setActivePinia(createPinia())
    Object.assign(core, { items: [], calls: [], changed: null, off: 0, getReply: null, next: 1 })
  })

  it('loads the core list and reloads when core says it changed', async () => {
    core.items = [{ id: 'a', title: 'A', body: 'b', scope: 'all', updatedAt: 1 }]
    const store = useSavedPromptsStore()
    store.start()
    await settle()
    expect(store.loaded).toBe(true)
    expect(store.items.map((p) => p.title)).toEqual(['A'])
    core.items = [...core.items, { id: 'b', title: 'B', body: 'b', scope: 'all', updatedAt: 2 }]
    core.changed?.()
    await settle()
    expect(store.items.map((p) => p.title)).toEqual(['A', 'B'])
  })

  it('subscribes once and unsubscribes on stop', async () => {
    const store = useSavedPromptsStore()
    store.start()
    store.start()
    await settle()
    store.stop()
    expect(core.off).toBe(1)
  })

  it('saves, removes and reorders through the core and keeps what it returns', async () => {
    const store = useSavedPromptsStore()
    store.start()
    await settle()
    const a = await store.save(draft('A'))
    const b = await store.save(draft('B'))
    expect(a.ok && b.ok).toBe(true)
    expect(store.items.map((p) => p.title)).toEqual(['A', 'B'])
    await store.reorder([store.items[1]!.id, store.items[0]!.id])
    expect(store.items.map((p) => p.title)).toEqual(['B', 'A'])
    await store.remove(store.items[0]!.id)
    expect(store.items.map((p) => p.title)).toEqual(['A'])
  })

  it('refuses a bad draft before it reaches the core', async () => {
    const store = useSavedPromptsStore()
    const sent = core.calls.length
    expect(await store.save({ title: ' ', body: 'x', scope: 'all' })).toEqual({ ok: false, error: 'invalid-title' })
    expect(await store.save({ title: 't', body: 'x'.repeat(4001), scope: 'all' })).toEqual({ ok: false, error: 'invalid-body' })
    expect(core.calls.length).toBe(sent)
  })

  it('keeps the old list and reports the error when core fails', async () => {
    core.items = [{ id: 'a', title: 'A', body: 'b', scope: 'all', updatedAt: 1 }]
    const store = useSavedPromptsStore()
    await store.load()
    core.getReply = async () => ({ ok: false, error: 'io' })
    await store.load()
    expect(store.error).toBe('io')
    expect(store.items).toHaveLength(1)
  })

  it('ignores a slow load that answers after a newer save', async () => {
    const store = useSavedPromptsStore()
    let release: (v: unknown) => void = () => undefined
    core.getReply = () => new Promise((r) => (release = r))
    const slow = store.load()
    core.getReply = null
    await store.save(draft('New'))
    release({ ok: true, version: 1, items: [] })
    await slow
    expect(store.items.map((p) => p.title)).toEqual(['New'])
  })

  it('imports the old localStorage list once when the core list is empty', async () => {
    localStorage.setItem(LEGACY_KEY, JSON.stringify([{ id: 'x', name: 'Old', text: 'old text', scope: 'global' }]))
    const store = useSavedPromptsStore()
    await store.load()
    expect(store.items.map((p) => p.title)).toEqual(['Old'])
    expect(localStorage.getItem(MIGRATED_KEY)).toBe('1')
    setActivePinia(createPinia())
    await useSavedPromptsStore().load()
    expect(core.items).toHaveLength(1)
  })

  it('does not import over a core list that already has prompts', async () => {
    core.items = [{ id: 'a', title: 'A', body: 'b', scope: 'all', updatedAt: 1 }]
    localStorage.setItem(LEGACY_KEY, JSON.stringify([{ id: 'x', name: 'Old', text: 'old', scope: 'global' }]))
    await useSavedPromptsStore().load()
    expect(core.items).toHaveLength(1)
    expect(localStorage.getItem(MIGRATED_KEY)).toBe('1')
  })

  it('reports a recovered file once the core says so', async () => {
    core.getReply = async () => ({ ok: true, version: 1, items: [], recovered: true })
    const store = useSavedPromptsStore()
    await store.load()
    expect(store.recovered).toBe(true)
  })
})
