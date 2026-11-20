import { describe, expect, it } from 'vitest'
import type { ToolSpan } from '@ccm/shared'
import { buildTouchIndex, type TouchTerminal } from './agentTouches'
import {
  addIgnored,
  clashesByTerminal,
  clashKey,
  clashList,
  MAX_IGNORED,
  newClashes,
  othersOf,
  pairKeys,
  parseIgnored,
  pruneIgnored,
  relPath,
  removeIgnored,
  summarize,
  type Clash,
} from './clash'

const NOW = Date.parse('2026-11-20T10:00:00.000Z')
const span = (id: string, terminalId: string, path: string, access: 'edit' | 'read' = 'edit', offset = -1000): ToolSpan => ({
  id,
  terminalId,
  agentId: `${terminalId}:main`,
  tool: access === 'edit' ? 'Edit' : 'Read',
  start: new Date(NOW + offset).toISOString(),
  path,
  access,
})
const working = (name: string): TouchTerminal => ({ name, provider: 'claude', working: true, turnStart: NOW - 60_000 })
const terms = (o: Record<string, TouchTerminal>) => new Map(Object.entries(o))
const index = (spans: ToolSpan[], o: Record<string, TouchTerminal>) => buildTouchIndex(spans, terms(o), NOW)

describe('clashList', () => {
  it('lists only files two terminals edit, keeping the original path', () => {
    const idx = index(
      [span('1', 'a', 'F:/Repo/src/A.ts'), span('2', 'b', 'F:/repo/src/a.ts'), span('3', 'a', 'F:/Repo/src/solo.ts'), span('4', 'b', 'F:/Repo/src/read.ts', 'read'), span('5', 'a', 'F:/Repo/src/read.ts')],
      { a: working('Alpha'), b: working('Beta') },
    )
    const list = clashList(idx)
    expect(list.map((c) => c.fileKey)).toEqual(['f:/repo/src/a.ts'])
    expect(list[0]!.path).toBe('F:/Repo/src/A.ts')
    expect(list[0]!.peers.map((p) => p.name).sort()).toEqual(['Alpha', 'Beta'])
  })

  it('one agent editing twice is not a clash', () => {
    expect(clashList(index([span('1', 'a', '/p/a.ts'), span('2', 'a', '/p/a.ts', 'edit', -500)], { a: working('A') }))).toEqual([])
  })

  it('two worktrees with the same relative path do not clash', () => {
    expect(clashList(index([span('1', 'a', '/wt1/a.ts'), span('2', 'b', '/wt2/a.ts')], { a: working('A'), b: working('B') }))).toEqual([])
  })

  it('groups clashes per terminal', () => {
    const list = clashList(
      index([span('1', 'a', '/p/a.ts'), span('2', 'b', '/p/a.ts'), span('3', 'b', '/p/b.ts'), span('4', 'c', '/p/b.ts')], { a: working('A'), b: working('B'), c: working('C') }),
    )
    const by = clashesByTerminal(list)
    expect(by.get('a')?.map((c) => c.fileKey)).toEqual(['/p/a.ts'])
    expect(by.get('b')?.map((c) => c.fileKey)).toEqual(['/p/a.ts', '/p/b.ts'])
    expect(by.get('c')?.map((c) => c.fileKey)).toEqual(['/p/b.ts'])
    expect(othersOf(list[0]!, 'a').map((p) => p.terminalId)).toEqual(['b'])
  })
})

describe('newClashes', () => {
  const clash = (fileKey: string, ...ids: string[]): Clash => ({ fileKey, path: fileKey, peers: ids.map((id) => ({ terminalId: id, name: id, at: NOW })) })

  it('orders the pair key so A,B and B,A match', () => {
    expect(clashKey('/f', 'b', 'a')).toBe(clashKey('/f', 'a', 'b'))
  })

  it('reports a pair once per turn and again when one side starts a new turn', () => {
    const turns: Record<string, string> = { a: '1', b: '1' }
    const turnKey = (id: string) => turns[id] ?? ''
    const reported = new Set<string>()
    const next = [clash('/f', 'a', 'b')]
    expect(newClashes(reported, next, turnKey)).toHaveLength(1)
    for (const k of pairKeys(next[0]!, turnKey)) reported.add(k)
    expect(newClashes(reported, next, turnKey)).toHaveLength(0)
    turns.a = '2'
    expect(newClashes(reported, next, turnKey)).toHaveLength(1)
  })

  it('a third agent joining adds new pairs', () => {
    const turnKey = () => '1'
    const reported = new Set(pairKeys(clash('/f', 'a', 'b'), turnKey))
    expect(pairKeys(clash('/f', 'a', 'b', 'c'), turnKey)).toHaveLength(3)
    expect(newClashes(reported, [clash('/f', 'a', 'b', 'c')], turnKey)).toHaveLength(1)
  })
})

describe('ignored files', () => {
  it('keeps max 200 entries, dropping the oldest, and re-adding moves to the end', () => {
    let list = Array.from({ length: MAX_IGNORED }, (_, i) => ({ key: `k${i}`, ids: ['a'] }))
    list = addIgnored(list, { key: 'new', ids: ['a'] })
    expect(list).toHaveLength(MAX_IGNORED)
    expect(list[0]!.key).toBe('k1')
    list = addIgnored(list, { key: 'k5', ids: ['b'] })
    expect(list.at(-1)).toEqual({ key: 'k5', ids: ['b'] })
    expect(list.filter((x) => x.key === 'k5')).toHaveLength(1)
    expect(removeIgnored(list, 'k5').some((x) => x.key === 'k5')).toBe(false)
  })

  it('drops an entry once every terminal it named is gone', () => {
    const list = [
      { key: 'x', ids: ['a', 'b'] },
      { key: 'y', ids: ['c'] },
    ]
    expect(pruneIgnored(list, (id) => id === 'b').map((x) => x.key)).toEqual(['x'])
  })

  it('parses stored data defensively', () => {
    expect(parseIgnored(null)).toEqual([])
    expect(parseIgnored('nope')).toEqual([])
    expect(parseIgnored('{"key":1}')).toEqual([])
    expect(parseIgnored('[{"key":"a","ids":["t",3]},{"key":2}]')).toEqual([{ key: 'a', ids: ['t'] }])
  })
})

describe('labels', () => {
  it('shows the path relative to the session folder, else the file name', () => {
    expect(relPath('F:\\Work\\Repo\\src\\a.ts', 'f:/work/repo')).toBe('src/a.ts')
    expect(relPath('/other/place/b.ts', '/p')).toBe('b.ts')
    expect(relPath('/p/c.ts')).toBe('c.ts')
  })

  it('summarizes max 3 items and counts the rest', () => {
    expect(summarize([1, 2, 3, 4, 5])).toEqual({ shown: [1, 2, 3], more: 2 })
    expect(summarize([1])).toEqual({ shown: [1], more: 0 })
  })
})
