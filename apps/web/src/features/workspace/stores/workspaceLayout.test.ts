import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useWorkspaceStore } from './workspace'
import { leaves } from '../lib/splitLayout'
import { presetLayout } from '../lib/layoutPresets'
import { layoutSignature, snapshotLayout } from '../lib/layoutUndo'

function threePanes() {
  const ws = useWorkspaceStore()
  const a = ws.openFile('C:\\repo\\a.ts')
  const b = ws.openFile('C:\\repo\\b.ts', undefined, { split: { key: a.key, dir: 'row' } })
  const c = ws.openFile('C:\\repo\\c.ts', undefined, { split: { key: b.key, dir: 'col' } })
  return { ws, a: a.key, b: b.key, c: c.key }
}

describe('workspace layout presets and undo', () => {
  beforeEach(() => {
    localStorage.clear()
    sessionStorage.clear()
    setActivePinia(createPinia())
  })

  it('setLayout rearranges the same panes and refuses a different set', () => {
    const { ws, a, b, c } = threePanes()
    const tab = ws.tabs[0]!
    const objects = ws.panes.slice()
    const root = presetLayout('cols3', [a, b, c])!
    expect(ws.setLayout(tab.key, root)).toBe(true)
    expect(ws.tabs[0]!.root).toEqual(root)
    expect(ws.panes).toEqual(objects)
    expect(ws.setLayout(tab.key, presetLayout('cols2', [a, b])!)).toBe(false)
  })

  it('restoreLayout brings back the tabs as they were, panes untouched', () => {
    const { ws, b } = threePanes()
    const snap = snapshotLayout(ws.tabs, ws.panes, ws.activeKey)
    const panes = ws.panes.map((p) => p.key)
    ws.detachPane(b)
    expect(ws.tabs).toHaveLength(2)
    expect(layoutSignature(ws.tabs)).not.toBe(snap.sig)
    expect(ws.restoreLayout(snap)).toBe(true)
    expect(ws.tabs).toHaveLength(1)
    expect(layoutSignature(ws.tabs)).toBe(snap.sig)
    expect(ws.panes.map((p) => p.key)).toEqual(panes)
    expect(ws.activeKey).toBe(snap.active)
  })

  it('restoreLayout refuses when a pane was closed since', () => {
    const { ws, c } = threePanes()
    const snap = snapshotLayout(ws.tabs, ws.panes, ws.activeKey)
    ws.remove(c)
    expect(ws.restoreLayout(snap)).toBe(false)
    expect(leaves(ws.tabs[0]!.root)).not.toContain(c)
  })
})
