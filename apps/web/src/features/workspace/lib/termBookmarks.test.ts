import { describe, expect, it } from 'vitest'
import { BookmarkList, bookmarkLabel, type BookmarkMarker } from './termBookmarks'
import { historyKey, markKeyAction, workspaceAction, type TermKeyEvent } from './workspaceKeys'

class Mk implements BookmarkMarker {
  isDisposed = false
  constructor(public line: number) {}
  dispose(): void {
    this.isDisposed = true
  }
}

describe('BookmarkList', () => {
  it('adds, refuses a second bookmark on the same line and drops the spare marker', () => {
    const list = new BookmarkList<Mk>()
    expect(list.add('manual', new Mk(5))).not.toBeNull()
    const spare = new Mk(5)
    expect(list.add('error', spare, '1')).toBeNull()
    expect(spare.isDisposed).toBe(true)
    expect(list.items).toHaveLength(1)
  })

  it('finds the nearest bookmark above and below, and skips the one just jumped to', () => {
    const list = new BookmarkList<Mk>()
    const a = list.add('manual', new Mk(10))!
    const b = list.add('error', new Mk(40), '2')!
    const c = list.add('server', new Mk(90), 'localhost:5173')!
    expect(list.near(50, -1)).toBe(b)
    expect(list.near(50, 1)).toBe(c)
    expect(list.near(5, -1)).toBeNull()
    expect(list.near(95, 1)).toBeNull()
    expect(list.near(40, 1)).toBe(c)
    expect(list.near(100, -1)).toBe(c)
    expect(list.near(100, -1, c.id)).toBe(b)
    expect(list.near(0, 1)).toBe(a)
  })

  it('forgets a bookmark whose marker was disposed with the buffer', () => {
    const list = new BookmarkList<Mk>()
    const a = list.add('manual', new Mk(3))!
    a.marker.dispose()
    expect(list.sorted()).toEqual([])
    expect(list.near(10, -1)).toBeNull()
    expect(list.at(3)).toBeUndefined()
  })

  it('keeps at most max bookmarks, dropping the topmost, and clear disposes all', () => {
    const dropped: number[] = []
    const list = new BookmarkList<Mk>(3, (b) => dropped.push(b.marker.line))
    for (const line of [1, 2, 3, 4]) list.add('manual', new Mk(line))
    expect(list.sorted().map((b) => b.marker.line)).toEqual([2, 3, 4])
    expect(dropped).toEqual([1])
    const kept = list.items.map((b) => b.marker)
    list.clear()
    expect(list.items).toEqual([])
    expect(kept.every((m) => m.isDisposed)).toBe(true)
  })
})

describe('bookmarkLabel', () => {
  const t = (key: string, p?: Record<string, unknown>): string => `${key}:${JSON.stringify(p ?? {})}`
  it('labels each kind', () => {
    expect(bookmarkLabel('error', '1', t)).toBe('termMarks.error:{"code":"1"}')
    expect(bookmarkLabel('server', 'localhost:5173', t)).toBe('termMarks.server:{"host":"localhost:5173"}')
    expect(bookmarkLabel('manual', '', t)).toBe('termMarks.manual:{}')
  })
})

const key = (code: string, mods: Partial<TermKeyEvent> = {}, altGraph = false): TermKeyEvent => ({
  type: 'keydown',
  ctrlKey: false,
  shiftKey: false,
  altKey: false,
  metaKey: false,
  code,
  getModifierState: (m: string) => m === 'AltGraph' && altGraph,
  ...mods,
})

describe('terminal keys', () => {
  it('Ctrl+Alt+M toggles and Ctrl+Alt+Up / Down jump', () => {
    const ca = { ctrlKey: true, altKey: true }
    expect(markKeyAction(key('KeyM', ca), false)).toEqual({ t: 'toggle' })
    expect(markKeyAction(key('ArrowUp', ca), false)).toEqual({ t: 'jump', dir: -1 })
    expect(markKeyAction(key('ArrowDown', ca), true)).toEqual({ t: 'jump', dir: 1 })
  })

  it('ignores other modifiers, AltGr, composing and key-up', () => {
    expect(markKeyAction(key('KeyM', { ctrlKey: true }), false)).toBeNull()
    expect(markKeyAction(key('KeyM', { altKey: true }), false)).toBeNull()
    expect(markKeyAction(key('KeyM', { ctrlKey: true, altKey: true, shiftKey: true }), false)).toBeNull()
    expect(markKeyAction(key('KeyM', { ctrlKey: true, altKey: true, metaKey: true }), false)).toBeNull()
    expect(markKeyAction(key('KeyM', { ctrlKey: true, altKey: true }, true), false)).toBeNull()
    expect(markKeyAction(key('KeyM', { ctrlKey: true, altKey: true, isComposing: true }), false)).toBeNull()
    expect(markKeyAction(key('KeyM', { ctrlKey: true, altKey: true, type: 'keyup' }), false)).toBeNull()
    expect(markKeyAction(key('ArrowUp', { ctrlKey: true }), false)).toBeNull()
  })

  it('only a plain Ctrl+R opens the history', () => {
    expect(historyKey(key('KeyR', { ctrlKey: true }), false)).toBe(true)
    expect(historyKey(key('KeyR', { ctrlKey: true }), true)).toBe(true)
    expect(historyKey(key('KeyR'), false)).toBe(false)
    expect(historyKey(key('KeyR', { ctrlKey: true, shiftKey: true }), false)).toBe(false)
    expect(historyKey(key('KeyR', { ctrlKey: true, altKey: true }), false)).toBe(false)
    expect(historyKey(key('KeyR', { metaKey: true }), true)).toBe(false)
    expect(historyKey(key('KeyR', { ctrlKey: true, type: 'keyup' }), false)).toBe(false)
  })

  it('does not collide with workspace shortcuts', () => {
    const ev = (code: string, mods: Partial<KeyboardEvent>) => ({ ...key(code), ...mods }) as unknown as KeyboardEvent
    for (const mac of [false, true]) {
      expect(workspaceAction(ev('KeyM', { ctrlKey: true, altKey: true }), mac, true, true, true)).toBeNull()
      expect(workspaceAction(ev('ArrowUp', { ctrlKey: true, altKey: true }), mac, true, true, true)).toBeNull()
      expect(workspaceAction(ev('ArrowDown', { ctrlKey: true, altKey: true }), mac, true, true, true)).toBeNull()
      expect(workspaceAction(ev('KeyR', { ctrlKey: true }), mac, true, true, true)).toBeNull()
    }
  })
})
