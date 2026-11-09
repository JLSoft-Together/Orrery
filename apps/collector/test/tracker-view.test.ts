import { describe, expect, it } from 'vitest'
import { emptyView, mergeView, type SubView } from '../src/trackerView'

const sub = (agentId: string, extra: Partial<SubView> = {}): SubView => ({ agentId, meta: { agentType: 'Explore' }, status: 'working', resolved: false, state: { peakContext: 0, codexCalls: [], currentTool: 'Grep', lastAt: 'a' }, ...extra })

describe('mergeView', () => {
  it('replaces on full, patches fields on a delta and keeps untouched objects', () => {
    const limitHit = { at: 'x' }
    const tokens = { input: 1, output: 2, cacheRead: 0 }
    const a = sub('a1')
    const b = sub('b1')
    let v = mergeView(emptyView(), { full: true, main: { peakContext: 5, codexCalls: [], compactions: 0, limitHit, tokens, currentTool: 'Bash' }, subs: [a, b] })
    expect(v.subs).toEqual([a, b])
    v = mergeView(v, { main: { currentTool: null, lastAt: 'b' }, subs: [{ agentId: 'b1', status: 'completed', resolved: true, completedAt: 'c', state: { currentTool: null, lastAt: 'c' } }, sub('c1')] })
    expect(v.main.limitHit).toBe(limitHit)
    expect(v.main.tokens).toBe(tokens)
    expect('currentTool' in v.main).toBe(false)
    expect(v.main.lastAt).toBe('b')
    expect(v.subs[0]).toBe(a)
    expect(v.subs[1]).toEqual({ ...b, status: 'completed', resolved: true, completedAt: 'c', state: { peakContext: 0, codexCalls: [], lastAt: 'c' } })
    expect(v.subs[1]!.meta).toBe(b.meta)
    expect(v.subs.map((s) => s.agentId)).toEqual(['a1', 'b1', 'c1'])
    const same = mergeView(v, { main: { lastAt: 'z' }, subs: [] })
    expect(same.subs).toBe(v.subs)
  })
})
