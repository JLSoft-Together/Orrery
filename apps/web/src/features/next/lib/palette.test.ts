import { describe, expect, it } from 'vitest'
import { bucketOf, compareSessions, cycleNextScope, groupOfSession, inNextScope, parseNextPrefix, repoName, sessionTerms, NEXT_GROUP_ORDER } from './palette'
import { canUndoLayout, registerLayoutUndo, undoLayout } from './paletteHooks'
import { matchText, searchable } from '@features/palette'

describe('next palette', () => {
  it('buckets sessions', () => {
    expect(bucketOf({ status: 'waiting' })).toBe('needs')
    expect(bucketOf({ status: 'error' })).toBe('needs')
    expect(bucketOf({ status: 'idle', attention: 'waiting' })).toBe('needs')
    expect(bucketOf({ status: 'working' })).toBe('running')
    expect(bucketOf({ status: 'idle', attention: 'done' })).toBe('doneUnseen')
    expect(bucketOf({ status: 'idle' })).toBe('recent')
    expect(bucketOf({ status: 'stale' })).toBe('recent')
  })

  it('orders by bucket then newest first', () => {
    const list = [
      { id: 'a', bucket: 'recent' as const, since: '2026-11-15T10:00:00Z' },
      { id: 'b', bucket: 'needs' as const, since: '2026-11-15T08:00:00Z' },
      { id: 'c', bucket: 'recent' as const, since: '2026-11-15T11:00:00Z' },
      { id: 'd', bucket: 'running' as const },
    ]
    expect(list.sort(compareSessions).map((x) => x.id)).toEqual(['b', 'd', 'c', 'a'])
  })

  it('puts the longest waiting first in needs', () => {
    const list = [
      { id: 'a', bucket: 'needs' as const, since: '2026-11-15T10:00:00Z' },
      { id: 'b', bucket: 'needs' as const, since: '2026-11-15T08:00:00Z' },
      { id: 'c', bucket: 'needs' as const, since: '2026-11-15T09:00:00Z' },
    ]
    expect(list.sort(compareSessions).map((x) => x.id)).toEqual(['b', 'c', 'a'])
  })

  it('empty state order puts sessions before commands before projects', () => {
    const at = (g: Parameters<typeof NEXT_GROUP_ORDER.indexOf>[0]) => NEXT_GROUP_ORDER.indexOf(g)
    expect(at('needs')).toBeLessThan(at('running'))
    expect(at('running')).toBeLessThan(at('doneUnseen'))
    expect(at('doneUnseen')).toBeLessThan(at('recent'))
    expect(at('recent')).toBeLessThan(at('actions'))
    expect(at('actions')).toBeLessThan(at('projects'))
  })

  it('keeps session buckets only while the query is empty', () => {
    expect(groupOfSession('needs', false)).toBe('needs')
    expect(groupOfSession('needs', true)).toBe('sessions')
  })

  it('parses prefixes', () => {
    expect(parseNextPrefix('@abc')).toEqual({ mode: 'sessions', text: 'abc' })
    expect(parseNextPrefix('# file.ts')).toEqual({ mode: 'files', text: 'file.ts' })
    expect(parseNextPrefix('>theme')).toEqual({ mode: 'commands', text: 'theme' })
    expect(parseNextPrefix(': orrery')).toEqual({ mode: 'projects', text: 'orrery' })
    expect(parseNextPrefix(' plain ')).toEqual({ mode: null, text: 'plain' })
  })

  it('scopes', () => {
    expect(inNextScope('needs', 'sessions')).toBe(true)
    expect(inNextScope('settings', 'commands')).toBe(true)
    expect(inNextScope('favorites', 'projects')).toBe(true)
    expect(inNextScope('files', 'commands')).toBe(false)
    expect(inNextScope('files', 'all')).toBe(true)
    expect(cycleNextScope('all', -1)).toBe('projects')
    expect(cycleNextScope('projects', 1)).toBe('all')
  })

  it('matches model, branch and repo', () => {
    expect(repoName('F:\\work\\web\\Orrery')).toBe('Orrery')
    expect(repoName('/home/a/proj/')).toBe('proj')
    expect(repoName(undefined)).toBeUndefined()
    const terms = sessionTerms({ model: 'Opus 5.5', branch: 'feat_mist', cwd: 'F:\\work\\web\\Orrery' })
    const items = [searchable('Fix login', ...terms), searchable('Other', 'Sonnet 5', 'main', 'billing')]
    expect(matchText(items, 'opus').map((x) => x.label)).toEqual(['Fix login'])
    expect(matchText(items, 'feat_mist').map((x) => x.label)).toEqual(['Fix login'])
    expect(matchText(items, 'orrery').map((x) => x.label)).toEqual(['Fix login'])
    expect(matchText(items, 'billing').map((x) => x.label)).toEqual(['Other'])
  })

  it('layout undo hook is safe when empty and runs when registered', () => {
    expect(canUndoLayout()).toBe(false)
    expect(undoLayout()).toBe(false)
    let n = 0
    const off = registerLayoutUndo(() => void n++)
    expect(canUndoLayout()).toBe(true)
    expect(undoLayout()).toBe(true)
    expect(n).toBe(1)
    off()
    expect(undoLayout()).toBe(false)
  })
})
