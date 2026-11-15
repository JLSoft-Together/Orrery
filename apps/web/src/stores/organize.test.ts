import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import type { OrganizeJunkItem, OrganizePlan, OrganizeState } from '@ccm/shared'

const api = vi.hoisted(() => ({
  state: vi.fn(),
  settings: vi.fn(),
  add: vi.fn(),
  plan: vi.fn(),
  run: vi.fn(),
  undo: vi.fn(),
  junk: vi.fn(),
  del: vi.fn(),
}))

vi.mock('../lib/organize', async (orig) => ({
  ...(await orig<typeof import('../lib/organize')>()),
  hasOrganize: true,
  organizeState: () => api.state(),
  organizeSettings: (p: unknown) => api.settings(p),
  organizeAddSource: (k: unknown) => api.add(k),
  organizeRemoveSource: async () => null,
  organizePlan: (s: unknown) => api.plan(s),
  organizeRun: (...a: unknown[]) => api.run(...a),
  organizeUndo: (r: unknown) => api.undo(r),
  organizeJunk: () => api.junk(),
  organizeDelete: (p: unknown) => api.del(p),
  onOrganizeChanged: () => () => undefined,
}))

import { useOrganizeStore } from './organize'
import { ruleValid } from '../lib/organize'

const state = (over: Partial<OrganizeState['settings']> = {}): OrganizeState => ({
  settings: { sources: ['C:\\Users\\u\\Downloads'], source: 'C:\\Users\\u\\Downloads', mode: 'type', kinds: ['image', 'doc'], rules: [], auto: false, ...over },
  sources: [{ path: 'C:\\Users\\u\\Downloads', label: 'downloads', ok: true }],
  runs: [],
  watching: [],
  stableMs: 120_000,
})

const plan: OrganizePlan = {
  id: 'p1',
  source: 'C:\\Users\\u\\Downloads',
  mode: 'type',
  items: [
    { id: '0', name: 'a.jpg', folder: 'Ảnh', target: 'a.jpg', size: 10, mtime: 1, kind: 'image' },
    { id: '1', name: 'b.pdf', folder: 'Tài liệu', target: 'b.pdf', size: 20, mtime: 1, kind: 'doc' },
  ],
  skipped: [{ name: 'c.crdownload', reason: 'downloading' }],
}

beforeEach(() => {
  setActivePinia(createPinia())
  for (const f of Object.values(api)) f.mockReset()
  api.state.mockResolvedValue(state())
})

describe('organize store', () => {
  it('previews, unticks a row and sends only the plan id and excluded ids', async () => {
    const s = useOrganizeStore()
    await s.init()
    api.plan.mockResolvedValue(plan)
    await s.preview()
    expect(api.plan).toHaveBeenCalledWith('C:\\Users\\u\\Downloads')
    expect(s.chosen).toHaveLength(2)
    s.toggleItem('0')
    expect(s.chosenBytes).toBe(20)
    api.run.mockResolvedValue({ run: 'r1', moved: 1, bytes: 20, failed: [] })
    await s.run()
    expect(api.run).toHaveBeenCalledWith('p1', ['0'])
    expect(s.plan).toBeNull()
    expect(s.result?.moved).toBe(1)
  })

  it('drops the preview when the settings change and keeps the source error', async () => {
    const s = useOrganizeStore()
    await s.init()
    api.plan.mockResolvedValue(plan)
    await s.preview()
    api.settings.mockResolvedValue(state({ mode: 'month' }))
    await s.setMode('month')
    expect(api.settings).toHaveBeenCalledWith({ mode: 'month' })
    expect(s.plan).toBeNull()
    await s.toggleKind('image')
    expect(api.settings).toHaveBeenLastCalledWith({ kinds: ['doc'] })
    api.add.mockResolvedValue({ state: state(), reason: 'protected' })
    await s.addSource('pick')
    expect(s.sourceError).toBe('protected')
  })

  it('saves only valid rules', async () => {
    const s = useOrganizeStore()
    await s.init()
    api.settings.mockResolvedValue(state())
    await s.saveRules([
      { kind: 'ext', value: ' .pdf ', folder: 'PDF' },
      { kind: 'ext', value: 'pdf', folder: '../x' },
      { kind: 'size', value: '0', folder: 'Big' },
    ])
    expect(api.settings).toHaveBeenCalledWith({ rules: [{ kind: 'ext', value: '.pdf', folder: 'PDF' }] })
    expect(ruleValid({ kind: 'name', value: 'hoá đơn', folder: 'Hoá đơn' })).toBe(true)
    expect(ruleValid({ kind: 'size', value: 5 as unknown as string, folder: 'Big' })).toBe(true)
  })

  it('undoes a run and reloads', async () => {
    const s = useOrganizeStore()
    await s.init()
    api.undo.mockResolvedValue({ restored: 2, conflicts: [{ name: 'x', reason: 'changed' }], dirs: 1 })
    await s.undo('r1')
    expect(api.undo).toHaveBeenCalledWith('r1')
    expect(s.undoResult).toMatchObject({ run: 'r1', restored: 2 })
    expect(api.state).toHaveBeenCalledTimes(2)
  })

  it('preselects broken items, never the unknown ones, and drops deleted rows', async () => {
    const items: OrganizeJunkItem[] = [
      { path: 'D:\\a', name: 'a', kind: 'empty', state: 'empty' },
      { path: 'D:\\b.lnk', name: 'b.lnk', kind: 'shortcut', state: 'broken', target: 'D:\\gone' },
      { path: 'D:\\c.lnk', name: 'c.lnk', kind: 'shortcut', state: 'unknown', target: '\\\\nas\\x' },
    ]
    api.junk.mockResolvedValue({ indexed: true, items })
    const s = useOrganizeStore()
    await s.scanJunk()
    expect([...s.junkPicked]).toEqual(['D:\\a', 'D:\\b.lnk'])
    api.del.mockResolvedValue({ done: [{ path: 'D:\\a', size: 0 }], failed: [], freed: 0, permanent: false })
    expect(await s.removeJunk()).toBe(1)
    expect(api.del).toHaveBeenCalledWith(['D:\\a', 'D:\\b.lnk'])
    expect(s.junk.map((i) => i.path)).toEqual(['D:\\b.lnk', 'D:\\c.lnk'])
  })
})
