import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { levelAllows, levelFor, parseLevels, withLevel } from './projectLevel'
import { useProjectNotifyStore } from './stores/projectNotify'
import { cleanQuestion, clip, trayPayload, TRAY_MAX_ROWS } from './trayWaiting'

describe('project notification level', () => {
  it('all lets every kind through, off none, waiting only the waiting kinds', () => {
    for (const k of ['waiting', 'reminder', 'job', 'done', 'context', 'stuck', 'loop', 'clash', 'limit']) {
      expect(levelAllows('all', k)).toBe(true)
      expect(levelAllows('off', k)).toBe(false)
    }
    expect(['waiting', 'reminder', 'job'].every((k) => levelAllows('waiting', k))).toBe(true)
    expect(['done', 'context', 'stuck', 'loop', 'clash'].some((k) => levelAllows('waiting', k))).toBe(false)
  })

  it('matches the longest project root, case and slash insensitive', () => {
    const map = withLevel(withLevel({}, 'C:/Work/app', 'off'), 'C:/Work/app/sub', 'waiting')
    expect(levelFor(map, 'c:/work/APP/lib')).toBe('off')
    expect(levelFor(map, 'C:/Work/app/sub/x')).toBe('waiting')
    expect(levelFor(map, 'C:/Work/app2')).toBe('all')
    expect(levelFor(map, undefined)).toBe('all')
  })

  it('setting all removes the entry; bad stored values are dropped', () => {
    expect(withLevel({ 'c:/a': 'off' }, 'C:/a/', 'all')).toEqual({})
    expect(parseLevels('{"c:/a":"off","c:/b":"x","c:/c":"all"}')).toEqual({ 'c:/a': 'off' })
    expect(parseLevels('nope')).toEqual({})
    expect(parseLevels(null)).toEqual({})
  })
})

describe('project notify store', () => {
  beforeEach(() => {
    localStorage.clear()
    setActivePinia(createPinia())
  })

  it('filters by the level of the terminal folder and persists it', () => {
    const s = useProjectNotifyStore()
    s.set('C:/w/a', 'off')
    s.set('C:/w/b', 'waiting')
    expect(s.allows('C:/w/a/x', 'waiting')).toBe(false)
    expect(s.allows('C:/w/b', 'done')).toBe(false)
    expect(s.allows('C:/w/b', 'waiting')).toBe(true)
    expect(s.allows('C:/w/c', 'done')).toBe(true)
    setActivePinia(createPinia())
    expect(useProjectNotifyStore().levelOf('C:/w/a')).toBe('off')
  })
})

describe('tray waiting rows', () => {
  const byId = {
    a: { provider: 'claude', cwd: 'C:/w/alpha' },
    b: { provider: 'codex', cwd: 'C:/w/beta' },
    c: { provider: 'claude', cwd: 'C:/w/gamma' },
  }

  it('keeps only sessions that need an answer, once each, minus muted projects', () => {
    const items = [
      { reason: 'menu' as const, terminalId: 'a', question: 'Run it?' },
      { reason: 'done' as const, terminalId: 'b' },
      { reason: 'permission' as const, terminalId: 'a', question: 'again' },
      { reason: 'question' as const, terminalId: 'c', question: 'muted' },
    ]
    const p = trayPayload(items, byId, (cwd) => !cwd?.endsWith('gamma'))
    expect(p.total).toBe(1)
    expect(p.rows).toEqual([{ id: 'a', agent: 'Claude', project: 'alpha', question: 'Run it?' }])
  })

  it('caps rows at 8 but reports the total', () => {
    const ids = Array.from({ length: 11 }, (_, i) => `t${i}`)
    const map = Object.fromEntries(ids.map((id) => [id, { provider: 'claude', cwd: `C:/w/${id}` }]))
    const p = trayPayload(ids.map((id) => ({ reason: 'menu' as const, terminalId: id })), map, () => true)
    expect(p.total).toBe(11)
    expect(p.rows).toHaveLength(TRAY_MAX_ROWS)
  })

  it('cuts the question at 60 characters, joins lines and masks secrets', () => {
    expect([...clip('x'.repeat(100))].length).toBe(60)
    expect(clip('short')).toBe('short')
    const q = cleanQuestion('Use token=abc123secret\nand sk-abcdefghijklmnopqrstuvwx to push?')
    expect(q).not.toContain('abc123secret')
    expect(q).not.toContain('sk-abcdef')
    expect(q).not.toContain('\n')
    expect([...q].length).toBeLessThanOrEqual(60)
    expect(cleanQuestion(undefined)).toBe('')
  })
})
