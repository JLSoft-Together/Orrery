import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import type { PromptScan } from '../lib/promptAttachments'
import { fromPath, useAttachmentsStore } from './attachments'

const scan = (images: number[], files: string[] = []): PromptScan => ({ found: true, images, files, text: '' })
const P = 'pane-1'

describe('attachments store', () => {
  beforeEach(() => {
    localStorage.clear()
    setActivePinia(createPinia())
  })

  it('adding a drop does not open the tray; only the owner opens it', () => {
    const s = useAttachmentsStore()
    s.add(P, [fromPath('C:/x/a.ts', 'drop')!])
    expect(s.open[P]).toBeFalsy()
    s.toggle(P)
    expect(s.open[P]).toBe(true)
  })

  it('binds a new [Image #N] to the image whose paste key went out', () => {
    const s = useAttachmentsStore()
    const [a] = s.add(P, [{ kind: 'image', name: 'shot.png', source: 'drop', path: 'C:/x/shot.png' }])
    expect(a!.state).toBe('inserting')
    s.patch(P, a!.id, { awaiting: Date.now() })
    s.reconcile(P, scan([1]), false, null)
    expect(s.list(P)[0]).toMatchObject({ name: 'shot.png', token: '[Image #1]', state: 'inserted', seen: true })
  })

  it('tokens put there in the terminal get their own rows; a pending paste names the image', () => {
    const s = useAttachmentsStore()
    const { boundPaste } = s.reconcile(P, scan([4], ['@src/a.ts']), false, { kind: 'image', name: 'Clipboard image', source: 'paste', thumb: 'data:x' })
    expect(boundPaste).toBe(true)
    expect(s.list(P).map((a) => [a.name, a.token, a.source])).toEqual([
      ['Clipboard image', '[Image #4]', 'paste'],
      ['a.ts', '@src/a.ts', 'terminal'],
    ])
  })

  it('reports image rows that arrived without pixels (pasted in the terminal, screenshot hand-off)', () => {
    const s = useAttachmentsStore()
    const first = s.reconcile(P, scan([1, 2]), false, { kind: 'image', name: 'Clipboard image', source: 'paste', thumb: 'data:t', view: 'data:v' })
    const rows = s.list(P)
    expect(first.blind).toEqual([rows[1]!.id])
    const second = s.reconcile(P, scan([1, 2, 3]), false, { kind: 'image', name: 'Screenshot', source: 'paste' })
    expect(second.blind).toEqual([s.list(P)[2]!.id])
    expect(s.reconcile(P, scan([1, 2, 3]), false, null).blind).toEqual([])
  })

  it('removed in the terminal → gone; submitted → last sent, replaced by the next send', () => {
    const s = useAttachmentsStore()
    s.reconcile(P, scan([1, 2]), false, null)
    s.reconcile(P, scan([1]), false, null)
    expect(s.list(P).map((a) => a.token)).toEqual(['[Image #1]'])
    s.reconcile(P, scan([]), true, null)
    expect(s.sent(P).map((a) => a.token)).toEqual(['[Image #1]'])
    s.reconcile(P, scan([3]), false, null)
    s.reconcile(P, scan([]), true, null)
    expect(s.sent(P).map((a) => a.token)).toEqual(['[Image #3]'])
  })

  it('staged items wait; a screen without the input box changes nothing', () => {
    const s = useAttachmentsStore()
    s.autoInsert = false
    s.add(P, [fromPath('D:/p/readme.md', 'pick')])
    s.reconcile(P, { found: false, images: [], files: [], text: '' }, true, null)
    expect(s.list(P)[0]).toMatchObject({ kind: 'file', name: 'readme.md', state: 'staged' })
    s.insert(P)
    expect(s.list(P)[0]!.state).toBe('inserting')
    s.patch(P, s.list(P)[0]!.id, { state: 'inserted', token: '@readme.md' })
    // Not seen yet: a read before the paste lands must not drop it.
    s.reconcile(P, scan([]), false, null)
    expect(s.list(P)).toHaveLength(1)
    s.reconcile(P, scan([], ['@readme.md']), false, null)
    expect(s.list(P)[0]!.seen).toBe(true)
  })

  it('a mention being erased at the cursor leaves no ghost rows', () => {
    const s = useAttachmentsStore()
    s.reconcile(P, scan([], ['@package.json']), false, null)
    expect(s.list(P)).toHaveLength(1)
    s.reconcile(P, scan([], ['@package.jso']), false, null, '@package.jso')
    s.reconcile(P, scan([], ['@package.js']), false, null, '@package.js')
    expect(s.list(P)).toEqual([])
    // Finished (a space typed after it): the mention gets its row.
    s.reconcile(P, scan([], ['@src/b.ts']), false, null, null)
    expect(s.list(P).map((a) => a.token)).toEqual(['@src/b.ts'])
  })
})

describe('attachments reconcile no-op', () => {
  beforeEach(() => {
    localStorage.clear()
    setActivePinia(createPinia())
  })

  it('keeps byPane when the screen read changes nothing', () => {
    const s = useAttachmentsStore()
    s.reconcile(P, scan([1], ['@src/a.ts']), false, null)
    const before = s.byPane
    const list = s.list(P)
    s.reconcile(P, scan([1], ['@src/a.ts']), false, null)
    s.reconcile(P, scan([1], ['@src/a.ts']), false, null)
    expect(s.byPane).toBe(before)
    expect(s.list(P)).toBe(list)
  })

  it('still replaces byPane when an item changes', () => {
    const s = useAttachmentsStore()
    s.reconcile(P, scan([1]), false, null)
    const before = s.byPane
    s.reconcile(P, scan([1, 2]), false, null)
    expect(s.byPane).not.toBe(before)
    expect(s.list(P)).toHaveLength(2)
  })
})
