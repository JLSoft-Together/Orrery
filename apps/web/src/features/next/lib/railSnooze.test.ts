import { beforeEach, describe, expect, it, vi } from 'vitest'

describe('railSnooze', () => {
  let mod: typeof import('./railSnooze')

  beforeEach(async () => {
    sessionStorage.clear()
    vi.resetModules()
    mod = await import('./railSnooze')
  })

  it('persists to sessionStorage and survives a reload, pruning expired entries', async () => {
    mod.snooze('p1', 'question', 15)
    expect(JSON.parse(sessionStorage.getItem('ccm.railSnooze')!).until.p1).toBeGreaterThan(Date.now())
    vi.resetModules()
    mod = await import('./railSnooze')
    expect(mod.isSnoozed('p1')).toBe(true)
    sessionStorage.setItem('ccm.railSnooze', JSON.stringify({ until: { old: Date.now() - 1000, live: Date.now() + 60_000 }, since: { old: 1, live: 2 } }))
    vi.resetModules()
    mod = await import('./railSnooze')
    expect(mod.isSnoozed('old')).toBe(false)
    expect(mod.isSnoozed('live')).toBe(true)
  })

  it('ignores a corrupt stored value', async () => {
    sessionStorage.setItem('ccm.railSnooze', '{nope')
    vi.resetModules()
    mod = await import('./railSnooze')
    expect(mod.isSnoozed('x')).toBe(false)
  })

  it('records the wait it was snoozed for and releases it when the question changes', () => {
    let at: number | null | undefined = 100
    mod.setSnoozeSinceResolver(() => at)
    mod.snooze('p1', 'question', 60)
    mod.releaseChanged(() => 100)
    expect(mod.isSnoozed('p1')).toBe(true)
    mod.releaseChanged(() => undefined)
    expect(mod.isSnoozed('p1')).toBe(true)
    mod.releaseChanged(() => 200)
    expect(mod.isSnoozed('p1')).toBe(false)
    at = 100
    mod.snooze('p2', 'question', 15)
    mod.releaseChanged(() => null)
    expect(mod.isSnoozed('p2')).toBe(false)
  })

  it('keeps a snooze without a known since, and undo restores the earlier one', () => {
    mod.snooze('p1', 'question', 15)
    mod.releaseChanged(() => 999)
    expect(mod.isSnoozed('p1')).toBe(true)
    const second = mod.snooze('p1', 'question', 60)
    second.undo()
    expect(mod.isSnoozed('p1')).toBe(true)
    expect(mod.clearSnoozed('p1')).toBe(true)
    expect(mod.isSnoozed('p1')).toBe(false)
    expect(mod.clearSnoozed('p1')).toBe(false)
  })

  it('caps an error at 15 minutes and a question at 60', () => {
    expect(mod.snooze('e', 'error', 60).minutes).toBe(15)
    expect(mod.snooze('q', 'question', 60).minutes).toBe(60)
  })
})
