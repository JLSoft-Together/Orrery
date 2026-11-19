import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import type { CleanCategory, CleanItem } from '@ccm/shared'
import { pickable, preselect } from '../lib/diskClean'
import { errorKey, percent } from '../lib/diskMove'

const api = vi.hoisted(() => ({
  list: vi.fn(),
  state: vi.fn(),
  volumes: vi.fn(),
  plan: vi.fn(),
  run: vi.fn(),
  back: vi.fn(),
  recover: vi.fn(),
  fromClean: vi.fn(),
  command: vi.fn(),
}))

vi.mock('../lib/diskMove', async (orig) => ({
  ...(await orig<typeof import('../lib/diskMove')>()),
  hasDiskMove: true,
  diskMoveList: () => api.list(),
  diskMoveState: () => api.state(),
  diskMoveVolumes: (...a: unknown[]) => api.volumes(...a),
  diskMovePlan: (...a: unknown[]) => api.plan(...a),
  diskMoveRun: (...a: unknown[]) => api.run(...a),
  diskMoveBack: (...a: unknown[]) => api.back(...a),
  diskMoveRecover: (...a: unknown[]) => api.recover(...a),
  diskMoveFromClean: (...a: unknown[]) => api.fromClean(...a),
  diskMovePick: async () => null,
  diskMoveCancel: async () => true,
  onDiskMoveProgress: () => () => undefined,
}))

vi.mock('../lib/diskClean', async (orig) => ({
  ...(await orig<typeof import('../lib/diskClean')>()),
  hasDiskClean: true,
  diskCleanCommand: (...a: unknown[]) => api.command(...a),
  onDiskCleanProgress: () => () => undefined,
}))

import { useDiskMoveStore } from './diskMove'
import { useDiskCleanStore } from './diskClean'

const DAY = 86_400_000
const now = Date.now()
const G = { id: 'c1', path: 'C:\\U\\.gradle', kind: 'gradle' as const }
const VOLS = [
  { mount: 'C:\\', label: 'C', free: 10, total: 100, ok: false, reason: 'same' },
  { mount: 'D:\\', label: 'D', free: 1e12, total: 2e12, ok: true },
]

beforeEach(() => {
  setActivePinia(createPinia())
  for (const f of Object.values(api)) f.mockReset()
  api.list.mockResolvedValue([G, { id: 'c2', path: 'C:\\U\\Docker', kind: 'docker', guide: true }])
  api.state.mockResolvedValue({ moves: [], pending: [], running: null })
  api.volumes.mockResolvedValue(VOLS)
})

describe('sdk and docker rows in Clean up', () => {
  it('command rows and held rows are never ticked', () => {
    const cats: CleanCategory[] = ['sdk', 'docker'].map((id) => ({ id, group: 'dev', preselect: true, titleKey: '', hintKey: '' }))
    const items: CleanItem[] = [
      { path: 'C:\\sdk\\platforms\\android-30', size: 1, group: 'dev', category: 'sdk', kind: 'platform', lastTouched: now - DAY },
      { path: 'C:\\sdk\\platforms\\android-31', size: 1, group: 'dev', category: 'sdk', kind: 'platform', lastTouched: now - 90 * DAY, hold: true },
      { path: 'docker:images', size: 1, group: 'dev', category: 'docker', kind: 'images', action: 'command' },
    ]
    expect(pickable(items[2]!, [])).toBe(false)
    expect([...preselect(items, cats, 30, now, [])]).toEqual(['C:\\sdk\\platforms\\android-30'])
  })

  it('runs a command and lowers the row by what was freed', async () => {
    const clean = useDiskCleanStore()
    clean.items = [{ path: 'docker:images', size: 1000, group: 'dev', category: 'docker', kind: 'images', action: 'command', files: 3 }]
    api.command.mockResolvedValue({ done: true, freed: 600 })
    await clean.runCommand('docker:images')
    expect(clean.items[0]).toMatchObject({ size: 400, files: 0 })
    expect(clean.report).toMatchObject({ done: 1, freed: 600 })
    api.command.mockResolvedValue({ done: false, freed: 0, cancelled: true })
    await clean.runCommand('docker:images')
    expect(clean.report).toMatchObject({ done: 1, freed: 600 })
  })
})

describe('move store', () => {
  it('chooses the first usable drive, plans, runs and refreshes', async () => {
    const s = useDiskMoveStore()
    await s.init()
    expect(s.candidates).toHaveLength(2)
    await s.choose('c1')
    expect(s.volume).toBe('D:\\')
    api.plan.mockResolvedValue({ plan: { id: 'p1', candidate: 'c1', source: G.path, target: 'D:\\OrreryMoved\\.gradle', bytes: 100, files: 2, volume: 'D:\\', need: 110, free: 1e12 } })
    await s.makePlan()
    expect(api.plan).toHaveBeenCalledWith('c1', 'D:\\')
    expect(s.plan?.id).toBe('p1')
    api.run.mockResolvedValue({ ok: true, record: { id: 'm1' } })
    api.state.mockResolvedValue({ moves: [{ id: 'm1', source: G.path, target: 'D:\\OrreryMoved\\.gradle', at: now, bytes: 100, files: 2, state: 'moved' }], pending: [], running: null })
    await s.run()
    expect(api.run).toHaveBeenCalledWith('p1')
    expect(s.notice).toMatchObject({ ok: true, kind: 'move' })
    expect(s.moves).toHaveLength(1)
    expect(s.chosen).toBeNull()
  })

  it('keeps a plan error and the tool holding the folder', async () => {
    const s = useDiskMoveStore()
    await s.init()
    await s.choose('c1')
    api.plan.mockResolvedValue({ error: 'locked', holder: 'java.exe' })
    await s.makePlan()
    expect(s.plan).toBeNull()
    expect(s.notice).toMatchObject({ ok: false, error: 'locked', holder: 'java.exe' })
  })

  it('a cancelled native dialog leaves no notice', async () => {
    const s = useDiskMoveStore()
    await s.init()
    await s.choose('c1')
    api.plan.mockResolvedValue({ plan: { id: 'p1', candidate: 'c1', source: G.path, target: 'x', bytes: 1, files: 1, volume: 'D:\\', need: 2, free: 9 } })
    await s.makePlan()
    api.run.mockResolvedValue({ ok: false, cancelled: true })
    await s.run()
    expect(s.notice).toBeNull()
    expect(s.plan?.id).toBe('p1')
  })

  it('opens the panel from a Clean up row, or explains why not', async () => {
    const s = useDiskMoveStore()
    await s.init()
    api.fromClean.mockResolvedValue({ id: 'c9', path: 'C:\\x\\cache', kind: 'clean' })
    expect(await s.fromClean('C:\\x\\cache')).toBe(true)
    expect(s.chosen).toBe('c9')
    api.fromClean.mockResolvedValue({ id: 'c8', path: 'C:\\y', kind: 'clean', block: 'link' })
    await s.fromClean('C:\\y')
    expect(s.notice).toMatchObject({ error: 'link' })
    api.fromClean.mockResolvedValue(null)
    expect(await s.fromClean('C:\\z')).toBe(false)
  })

  it('maps errors and progress', () => {
    expect(errorKey('verify:hash')).toBe('verify')
    expect(errorKey('weird')).toBe('failed')
    expect(errorKey('targetMissing')).toBe('targetMissing')
    expect(percent({ bytes: 50, total: 200 })).toBe(25)
    expect(percent(null)).toBe(0)
  })
})
