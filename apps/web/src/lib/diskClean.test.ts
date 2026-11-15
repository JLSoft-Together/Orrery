import { afterEach, describe, expect, it } from 'vitest'
import type { CleanItem } from '@ccm/shared'
import { ageDays, clampDays, DAY_MS, DAYS_DEFAULT, DAYS_KEY, inProject, isStale, loadDays, loadFolders, lockReason, openRoots, preselect, relTo, saveDays, saveFolders, under } from './diskClean'

const NOW = 1_800_000_000_000
const item = (over: Partial<CleanItem>): CleanItem => ({ path: 'C:\\p\\node_modules', size: 10, group: 'dev', category: 'node_modules', project: 'C:\\p', ...over })

afterEach(() => localStorage.clear())

describe('paths', () => {
  it('under ignores case, slashes and trailing separators', () => {
    expect(under('C:\\Work\\App\\src', 'c:/work/app/')).toBe(true)
    expect(under('C:\\Work\\Apple', 'C:\\Work\\App')).toBe(false)
    expect(under('', 'C:\\')).toBe(false)
  })
  it('relTo shows the path inside the project', () => {
    expect(relTo('C:\\p\\android\\app\\build', 'C:\\p')).toBe('android\\app\\build')
    expect(relTo('D:\\x\\build', 'C:\\p')).toBe('D:\\x\\build')
  })
  it('inProject keeps items of the filtered project', () => {
    expect(inProject(item({}), 'C:\\p')).toBe(true)
    expect(inProject(item({}), 'C:\\q')).toBe(false)
    expect(inProject(item({ path: 'C:\\p\\sub\\build', project: 'C:\\p' }), 'C:\\p\\sub')).toBe(true)
    expect(inProject(item({}), null)).toBe(true)
  })
})

describe('locks', () => {
  it('locks a project with something open inside it, not a parent folder', () => {
    expect(lockReason(item({}), ['C:\\p\\lib'])).toBe('open')
    expect(lockReason(item({}), ['C:\\'])).toBeUndefined()
    expect(lockReason(item({}), ['C:\\other'])).toBeUndefined()
    expect(lockReason(item({ locked: 'self' }), [])).toBe('self')
    expect(lockReason(item({ project: undefined, path: 'C:\\cache', category: 'cache' }), ['C:\\cache\\x'])).toBe('open')
  })
  it('openRoots takes live terminals, running panes and live Run sessions', () => {
    expect(
      openRoots({
        terminals: [
          { cwd: 'C:\\a', status: 'working' },
          { cwd: 'C:\\b', status: 'stale' },
          { cwd: 'C:\\c', status: 'idle', endedAt: '2026' },
        ],
        panes: [
          { cwd: 'C:\\A', running: true },
          { cwd: 'C:\\d', running: false },
          { cwd: 'C:\\e', running: true },
        ],
        runs: [
          { cwd: 'C:\\f', live: true },
          { cwd: 'C:\\g', live: false },
        ],
      }),
    ).toEqual(['C:\\a', 'C:\\e', 'C:\\f'])
  })
})

describe('preselect', () => {
  const cats = [
    { id: 'node_modules', preselect: true },
    { id: 'build', preselect: true },
    { id: 'cache', preselect: false },
  ]
  it('ticks folders of projects untouched for more than the setting, never caches or locked ones', () => {
    const items = [
      item({ path: 'C:\\old\\node_modules', project: 'C:\\old', lastTouched: NOW - 60 * DAY_MS }),
      item({ path: 'C:\\new\\node_modules', project: 'C:\\new', lastTouched: NOW - 2 * DAY_MS }),
      item({ path: 'C:\\open\\build', project: 'C:\\open', category: 'build', lastTouched: NOW - 90 * DAY_MS }),
      item({ path: 'C:\\locked\\build', project: 'C:\\locked', category: 'build', lastTouched: NOW - 90 * DAY_MS, locked: 'self' }),
      item({ path: 'C:\\unknown\\build', project: 'C:\\unknown', category: 'build' }),
      item({ path: 'C:\\npm-cache', project: undefined, category: 'cache', lastTouched: NOW - 400 * DAY_MS }),
    ]
    expect([...preselect(items, cats, 30, NOW, ['C:\\open'])]).toEqual(['C:\\old\\node_modules'])
    expect([...preselect(items, cats, 1, NOW, [])].sort()).toEqual(['C:\\new\\node_modules', 'C:\\old\\node_modules', 'C:\\open\\build'])
  })
  it('isStale and ageDays', () => {
    expect(isStale({ lastTouched: NOW - 31 * DAY_MS }, 30, NOW)).toBe(true)
    expect(isStale({ lastTouched: NOW - 29 * DAY_MS }, 30, NOW)).toBe(false)
    expect(isStale({}, 30, NOW)).toBe(false)
    expect(ageDays(NOW - 3.5 * DAY_MS, NOW)).toBe(3)
    expect(ageDays(undefined, NOW)).toBeNull()
  })
})

describe('settings', () => {
  it('days are clamped and remembered under ccm.diskCleanDays', () => {
    expect(loadDays()).toBe(DAYS_DEFAULT)
    saveDays(90)
    expect(localStorage.getItem(DAYS_KEY)).toBe('90')
    expect(loadDays()).toBe(90)
    expect(clampDays(0)).toBe(1)
    expect(clampDays('abc')).toBe(DAYS_DEFAULT)
    expect(clampDays(99999)).toBe(3650)
  })
  it('folders survive a reload and junk storage is ignored', () => {
    saveFolders(['D:\\code'])
    expect(loadFolders()).toEqual(['D:\\code'])
    localStorage.setItem('ccm.diskCleanFolders', '{bad')
    expect(loadFolders()).toEqual([])
  })
})
