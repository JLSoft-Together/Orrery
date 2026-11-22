import { describe, expect, it } from 'vitest'
import { isSeparator, type MenuAction } from '../../../core/lib/menu'
import { paneLayoutEntries, paneLayoutId, paneLayoutKeys } from './layoutPaneMenu'
import { workspaceAction } from './workspaceKeys'
import { detachShortcutText } from '../../shortcuts/lib/shortcuts'

const labels = { splitRight: 'R', splitDown: 'D', focus: 'F', swap: 'S', zoom: 'Z', unzoom: 'U', detach: 'T', close: 'C' }
const ids = (list: ReturnType<typeof paneLayoutEntries>): string[] => list.filter((e): e is MenuAction => !isSeparator(e)).map((e) => e.id)

function key(code: string, mods: Partial<Pick<KeyboardEvent, 'ctrlKey' | 'altKey' | 'shiftKey' | 'metaKey'>>): KeyboardEvent {
  return { type: 'keydown', isComposing: false, code, ctrlKey: false, altKey: false, shiftKey: false, metaKey: false, getModifierState: () => false, ...mods } as unknown as KeyboardEvent
}

describe('pane layout menu', () => {
  it('lists the existing chords for both platforms', () => {
    expect(paneLayoutKeys(false)).toEqual({
      splitRight: 'Alt+Shift++',
      splitDown: 'Alt+Shift+-',
      focus: 'Alt+←↑↓→',
      swap: 'Alt+Shift+←↑↓→',
      zoom: 'Ctrl+Shift+Enter',
      detach: 'Ctrl+Alt+T',
      close: 'Ctrl+Shift+W',
    })
    expect(paneLayoutKeys(true).detach).toBe('⌥⌘T')
    expect(paneLayoutKeys(true).splitRight).toBe('⌘D')
  })

  it('shows focus / swap / zoom / detach only when they apply', () => {
    expect(ids(paneLayoutEntries(labels, false, { multi: false, zoomed: false, canDetach: false }))).toEqual(['layout:splitRight', 'layout:splitDown', 'layout:close'])
    const multi = paneLayoutEntries(labels, false, { multi: true, zoomed: false, canDetach: true })
    expect(ids(multi)).toEqual(['layout:splitRight', 'layout:splitDown', 'layout:focus', 'layout:swap', 'layout:zoom', 'layout:detach', 'layout:close'])
    expect((multi.find((e) => !isSeparator(e) && e.id === 'layout:detach') as MenuAction).shortcut).toBe('Ctrl+Alt+T')
    const zoomed = paneLayoutEntries(labels, false, { multi: false, zoomed: true, canDetach: true })
    expect((zoomed.find((e) => !isSeparator(e) && e.id === 'layout:zoom') as MenuAction).label).toBe('U')
  })

  it('parses ids', () => {
    expect(paneLayoutId('layout:swap')).toBe('swap')
    expect(paneLayoutId('layout:nope')).toBeNull()
    expect(paneLayoutId('restart')).toBeNull()
  })
})

describe('detach chord', () => {
  it('Ctrl+Alt+T on Windows / Linux', () => {
    expect(workspaceAction(key('KeyT', { ctrlKey: true, altKey: true }), false, true, false, true)).toEqual({ t: 'detachTab' })
    expect(workspaceAction(key('KeyT', { ctrlKey: true, altKey: true }), false, true)).toBeNull()
    expect(workspaceAction(key('KeyT', { ctrlKey: true, shiftKey: true }), false, true)).toEqual({ t: 'newTab' })
    expect(workspaceAction(key('KeyT', { ctrlKey: true, altKey: true, shiftKey: true }), false, true)).toBeNull()
  })

  it('Alt+Cmd+T on macOS, Cmd+T stays new tab', () => {
    expect(workspaceAction(key('KeyT', { metaKey: true, altKey: true }), true, true, false, true)).toEqual({ t: 'detachTab' })
    expect(workspaceAction(key('KeyT', { metaKey: true, altKey: true }), true, true)).toBeNull()
    expect(workspaceAction(key('KeyT', { metaKey: true }), true, true)).toEqual({ t: 'newTab' })
    expect(workspaceAction(key('KeyT', { metaKey: true, shiftKey: true }), true, true)).toEqual({ t: 'reopen' })
  })

  it('AltGr+T is left to the keyboard layout', () => {
    const e = { ...key('KeyT', { ctrlKey: true, altKey: true }), getModifierState: (k: string) => k === 'AltGraph' } as unknown as KeyboardEvent
    expect(workspaceAction(e, false, true)).toBeNull()
  })

  it('prints per platform', () => {
    expect(detachShortcutText(false)).toBe('Ctrl+Alt+T')
    expect(detachShortcutText(true)).toBe('⌥⌘T')
  })
})
