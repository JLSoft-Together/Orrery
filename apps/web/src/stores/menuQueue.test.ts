import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { TermMenu } from '../lib/termMenu'
import { nextInQueue, useMenuQueueStore } from './menuQueue'

const menu = (labels: string[] = ['Yes', 'No'], kind: TermMenu['kind'] = 'permission'): TermMenu => ({
  kind,
  agent: 'claude',
  items: labels.map((label, i) => ({ index: i + 1, label, row: 5 + i, selected: i === 0 })),
  highlighted: 0,
  numbered: true,
  confident: true,
  firstRow: 5,
  lastRow: 5 + labels.length - 1,
})

describe('menuQueue store', () => {
  beforeEach(() => setActivePinia(createPinia()))

  it('adds a pane when its menu appears and drops it when the menu is gone', () => {
    const q = useMenuQueueStore()
    q.report('a', { menu: menu(), question: 'Run it?' }, 10)
    expect(q.count).toBe(1)
    expect(q.entryOf('a')?.question).toBe('Run it?')
    q.report('a', null)
    expect(q.count).toBe(0)
  })

  it('orders by when the question started, ties by pane key', () => {
    const q = useMenuQueueStore()
    q.report('c', { menu: menu(), question: 'c' }, 30)
    q.report('a', { menu: menu(), question: 'a' }, 20)
    q.report('b', { menu: menu(), question: 'b' }, 20)
    expect(q.entries.map((e) => e.paneKey)).toEqual(['a', 'b', 'c'])
  })

  it('keeps the start time while the same question stays up (highlight moves)', () => {
    const q = useMenuQueueStore()
    q.report('a', { menu: menu(), question: 'Q' }, 10)
    const moved = menu()
    moved.items[0]!.selected = false
    moved.items[1]!.selected = true
    q.report('a', { menu: moved, question: 'Q' }, 99)
    expect(q.entryOf('a')?.since).toBe(10)
  })

  it('restarts the clock when the question or options change', () => {
    const q = useMenuQueueStore()
    q.report('a', { menu: menu(), question: 'Q1' }, 10)
    q.report('a', { menu: menu(), question: 'Q2' }, 50)
    expect(q.entryOf('a')?.since).toBe(50)
    q.report('a', { menu: menu(['A', 'B', 'C']), question: 'Q2' }, 70)
    expect(q.entryOf('a')?.since).toBe(70)
  })

  it('forgets a pane that is closed (unregister) and ignores unknown keys', () => {
    const q = useMenuQueueStore()
    const off = q.register('a', vi.fn())
    q.report('a', { menu: menu(), question: 'Q' }, 1)
    q.report('b', { menu: menu(), question: 'Q' }, 2)
    off()
    expect(q.entries.map((e) => e.paneKey)).toEqual(['b'])
    q.forget('zzz')
    expect(q.count).toBe(1)
  })

  it('answer goes to the picker of that pane only', async () => {
    const q = useMenuQueueStore()
    const a = vi.fn().mockResolvedValue(undefined)
    const b = vi.fn().mockResolvedValue(undefined)
    q.register('a', a)
    q.register('b', b)
    const item = menu().items[1]!
    await q.answer('b', item)
    expect(b).toHaveBeenCalledWith(item)
    expect(a).not.toHaveBeenCalled()
    await expect(q.answer('none', item)).resolves.toBeUndefined()
  })

  it('next steps through the queue and wraps', () => {
    const q = useMenuQueueStore()
    q.report('a', { menu: menu(), question: 'a' }, 1)
    q.report('b', { menu: menu(), question: 'b' }, 2)
    q.report('c', { menu: menu(), question: 'c' }, 3)
    expect(q.next(null)).toBe('a')
    expect(q.next('a')).toBe('b')
    expect(q.next('c')).toBe('a')
    expect(q.next('other')).toBe('a')
    q.report('a', null)
    q.report('b', null)
    q.report('c', null)
    expect(q.next('a')).toBeNull()
  })

  it('nextInQueue with a single entry stays on it', () => {
    expect(nextInQueue(['a'], 'a')).toBe('a')
    expect(nextInQueue([], null)).toBeNull()
  })
})
