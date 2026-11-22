import { beforeEach, describe, expect, it } from 'vitest'
import { workspaceAction } from '../../lib/workspaceKeys'
import { isPaletteKey } from '../../lib/paletteKey'
import { workspaceShortcuts } from '../../lib/shortcuts'
import { effectiveKeys, findConflict, isOverridden, overrides, recording, reloadOverrides, reservedReason, resetAllBindings, resetBinding, setBinding } from './bindings'
import { chordFromEvent, chordId, keysId, parseChord } from './keys'
import { mruOrder, mruStart, mruStep } from './mru'

const key = (code: string, mods: Partial<KeyboardEvent> = {}, altGraph = false): KeyboardEvent =>
  ({
    type: 'keydown',
    isComposing: false,
    ctrlKey: false,
    shiftKey: false,
    altKey: false,
    metaKey: false,
    code,
    key: code.replace(/^Key/, '').toLowerCase(),
    getModifierState: (m: string) => m === 'AltGraph' && altGraph,
    ...mods,
  }) as unknown as KeyboardEvent

beforeEach(() => {
  recording.value = false
  resetAllBindings()
})

describe('mru order', () => {
  it('puts the current tab first, then the most recently used', () => {
    const at: Record<string, number> = { a: 5, b: 30, c: 10, d: 0 }
    expect(mruOrder(['a', 'b', 'c', 'd'], 'a', (k) => at[k]!)).toEqual(['a', 'b', 'c', 'd'])
    expect(mruOrder(['a', 'b', 'c', 'd'], 'c', (k) => at[k]!)).toEqual(['c', 'b', 'a', 'd'])
  })

  it('keeps the tab order for ties', () => {
    expect(mruOrder(['x', 'y', 'z'], 'z', () => 0)).toEqual(['z', 'x', 'y'])
  })

  it('first press goes to the previous tab, Shift to the oldest, steps wrap', () => {
    expect(mruStart(5, 1)).toBe(1)
    expect(mruStart(5, -1)).toBe(4)
    expect(mruStep(4, 1, 5)).toBe(0)
    expect(mruStep(0, -1, 5)).toBe(4)
  })
})

describe('Ctrl+Tab', () => {
  it('opens the recent tab switcher on every platform, Shift goes back', () => {
    expect(workspaceAction(key('Tab', { ctrlKey: true }), false, false)).toEqual({ t: 'mru', step: 1 })
    expect(workspaceAction(key('Tab', { ctrlKey: true, shiftKey: true }), false, false)).toEqual({ t: 'mru', step: -1 })
    expect(workspaceAction(key('Tab', { ctrlKey: true }), true, false)).toEqual({ t: 'mru', step: 1 })
    expect(workspaceAction(key('Tab', { ctrlKey: true, altKey: true }), false, false)).toBeNull()
    expect(workspaceAction(key('Tab', { metaKey: true }), true, false)).toBeNull()
  })

  it('Ctrl+PageUp / PageDown keep walking the tab order', () => {
    expect(workspaceAction(key('PageDown', { ctrlKey: true }), false, false)).toEqual({ t: 'cycleTab', step: 1 })
    expect(workspaceAction(key('PageUp', { ctrlKey: true }), false, false)).toEqual({ t: 'cycleTab', step: -1 })
    expect(workspaceAction(key('PageUp', { ctrlKey: true, shiftKey: true }), false, false)).toEqual({ t: 'moveTab', step: -1 })
  })
})

describe('chords from events', () => {
  it('Ctrl on Windows stays Ctrl, Command on macOS becomes Mod and Control stays Ctrl', () => {
    expect(chordFromEvent(key('KeyT', { ctrlKey: true, shiftKey: true }), false)).toEqual({ mods: ['Ctrl', 'Shift'], code: 'KeyT' })
    expect(chordFromEvent(key('KeyT', { metaKey: true, shiftKey: true }), true)).toEqual({ mods: ['Shift', 'Mod'], code: 'KeyT' })
    expect(chordFromEvent(key('KeyT', { ctrlKey: true }), true)).toEqual({ mods: ['Ctrl'], code: 'KeyT' })
    expect(chordFromEvent(key('KeyT', { metaKey: true }), false)).toBeNull()
  })

  it('ignores modifier-only presses, AltGr and unknown keys', () => {
    expect(chordFromEvent(key('ControlLeft', { ctrlKey: true }), false)).toBeNull()
    expect(chordFromEvent(key('KeyT', { ctrlKey: true, altKey: true }, true), false)).toBeNull()
    expect(chordFromEvent(key('IntlBackslash', { ctrlKey: true }), false)).toBeNull()
  })

  it('Mod and Ctrl are the same key on Windows, different on macOS', () => {
    expect(keysId(['Mod', 'K'], false)).toBe(keysId(['Ctrl', 'K'], false))
    expect(keysId(['Mod', 'K'], true)).not.toBe(keysId(['Ctrl', 'K'], true))
    expect(chordId(parseChord(['Shift', 'Ctrl', 'T'], false)!)).toBe('Ctrl+Shift:KeyT')
  })
})

describe('reserved keys', () => {
  it('rejects bare keys, Shift only and terminal keys', () => {
    expect(reservedReason(['T'], false)).toBe('noModifier')
    expect(reservedReason(['Shift', 'T'], false)).toBe('noModifier')
    expect(reservedReason(['Ctrl', 'C'], false)).toBe('terminal')
    expect(reservedReason(['Mod', 'C'], false)).toBe('terminal')
    expect(reservedReason(['Ctrl', 'Shift', 'V'], false)).toBe('terminal')
    expect(reservedReason(['Alt', 'B'], false)).toBe('terminal')
    expect(reservedReason(['Ctrl', 'C'], true)).toBe('terminal')
    expect(reservedReason(['Ctrl'], false)).toBe('incomplete')
  })

  it('rejects system keys but allows normal chords', () => {
    expect(reservedReason(['Alt', 'Tab'], false)).toBe('system')
    expect(reservedReason(['Alt', 'F4'], false)).toBe('system')
    expect(reservedReason(['Mod', 'C'], true)).toBe('system')
    expect(reservedReason(['Mod', 'Q'], true)).toBe('system')
    expect(reservedReason(['Ctrl', 'Shift', 'J'], false)).toBeNull()
    expect(reservedReason(['Alt', 'Shift', 'J'], false)).toBeNull()
    expect(reservedReason(['Mod', 'Shift', 'J'], true)).toBeNull()
    expect(reservedReason(['F9'], false)).toBeNull()
  })
})

describe('rebinding', () => {
  it('a new chord works and the old default stops working', () => {
    expect(setBinding('wsNew', ['Ctrl', 'Shift', 'J'], false)).toEqual({ ok: true })
    expect(workspaceAction(key('KeyJ', { ctrlKey: true, shiftKey: true }), false, false)).toEqual({ t: 'newTab' })
    expect(workspaceAction(key('KeyT', { ctrlKey: true, shiftKey: true }), false, false)).toBeNull()
    expect(isOverridden('wsNew')).toBe(true)
    expect(effectiveKeys('wsNew', false)).toEqual(['Ctrl', 'Shift', 'J'])
    expect(workspaceShortcuts(false).find((s) => s.id === 'wsNew')?.keys).toEqual(['Ctrl', 'Shift', 'J'])
  })

  it('the same on macOS with Command', () => {
    expect(setBinding('wsNew', ['Mod', 'Shift', 'J'], true)).toEqual({ ok: true })
    expect(workspaceAction(key('KeyJ', { metaKey: true, shiftKey: true }), true, false)).toEqual({ t: 'newTab' })
    expect(workspaceAction(key('KeyT', { metaKey: true }), true, false)).toBeNull()
  })

  it('reports the command that already uses the chord and changes nothing', () => {
    expect(setBinding('wsNew', ['Ctrl', 'Shift', 'W'], false)).toEqual({ ok: false, reason: 'conflict', with: 'wsClose' })
    expect(isOverridden('wsNew')).toBe(false)
    expect(findConflict('wsNew', ['Mod', 'K'], false)).toBe('palette')
    setBinding('wsClose', ['Ctrl', 'Shift', 'J'], false)
    expect(setBinding('wsNew', ['Ctrl', 'Shift', 'J'], false)).toEqual({ ok: false, reason: 'conflict', with: 'wsClose' })
    expect(setBinding('wsNew', ['Ctrl', 'Shift', 'W'], false)).toEqual({ ok: true })
  })

  it('refuses terminal and system keys', () => {
    expect(setBinding('wsNew', ['Ctrl', 'C'], false)).toEqual({ ok: false, reason: 'terminal' })
    expect(setBinding('wsNew', ['Alt', 'Tab'], false)).toEqual({ ok: false, reason: 'system' })
    expect(setBinding('wsNew', ['T'], false)).toEqual({ ok: false, reason: 'noModifier' })
    expect(isOverridden('wsNew')).toBe(false)
  })

  it('reset one and reset all restore the defaults', () => {
    setBinding('wsNew', ['Ctrl', 'Shift', 'J'], false)
    setBinding('wsZoom', ['Ctrl', 'Shift', 'K'], false)
    resetBinding('wsNew')
    expect(workspaceAction(key('KeyT', { ctrlKey: true, shiftKey: true }), false, false)).toEqual({ t: 'newTab' })
    expect(isOverridden('wsZoom')).toBe(true)
    resetAllBindings()
    expect(workspaceAction(key('Enter', { ctrlKey: true, shiftKey: true }), false, false)).toEqual({ t: 'zoom' })
    expect(Object.keys(overrides.value)).toEqual([])
  })

  it('setting the default chord again stores nothing', () => {
    setBinding('wsNew', ['Ctrl', 'Shift', 'T'], false)
    expect(isOverridden('wsNew')).toBe(false)
  })

  it('persists as ccm.shortcuts and survives a reload; bad data is dropped', () => {
    setBinding('wsNew', ['Ctrl', 'Shift', 'J'], false)
    expect(JSON.parse(localStorage.getItem('ccm.shortcuts')!)).toEqual({ wsNew: ['Ctrl', 'Shift', 'J'] })
    overrides.value = {}
    reloadOverrides()
    expect(effectiveKeys('wsNew', false)).toEqual(['Ctrl', 'Shift', 'J'])
    localStorage.setItem('ccm.shortcuts', JSON.stringify({ wsNew: 'x', nope: ['Ctrl', 'J'], wsZoom: ['Ctrl', 'Shift', 'K'] }))
    reloadOverrides()
    expect(Object.keys(overrides.value)).toEqual(['wsZoom'])
    resetAllBindings()
    expect(localStorage.getItem('ccm.shortcuts')).toBeNull()
  })

  it('Ctrl+Tab itself can move: the new chord opens the switcher, Shift goes back', () => {
    expect(setBinding('wsMru', ['Ctrl', 'Alt', 'J'], false)).toEqual({ ok: true })
    expect(workspaceAction(key('KeyJ', { ctrlKey: true, altKey: true }), false, false)).toEqual({ t: 'mru', step: 1 })
    expect(workspaceAction(key('KeyJ', { ctrlKey: true, altKey: true, shiftKey: true }), false, false)).toEqual({ t: 'mru', step: -1 })
    expect(workspaceAction(key('Tab', { ctrlKey: true }), false, false)).toBeNull()
  })

  it('the palette key can move too', () => {
    const k = (code: string, mods: Partial<KeyboardEvent>): KeyboardEvent => key(code, mods)
    expect(isPaletteKey(k('KeyK', { ctrlKey: true }), false, false)).toBe(true)
    expect(setBinding('palette', ['Ctrl', 'Shift', 'J'], false)).toEqual({ ok: true })
    expect(isPaletteKey(k('KeyK', { ctrlKey: true }), false, false)).toBe(false)
    expect(isPaletteKey(k('KeyJ', { ctrlKey: true, shiftKey: true }), false, false)).toBe(true)
  })

  it('shortcuts are ignored while a key is being recorded', () => {
    recording.value = true
    expect(workspaceAction(key('KeyT', { ctrlKey: true, shiftKey: true }), false, false)).toBeNull()
    expect(isPaletteKey(key('KeyK', { ctrlKey: true }), false, false)).toBe(false)
  })
})
