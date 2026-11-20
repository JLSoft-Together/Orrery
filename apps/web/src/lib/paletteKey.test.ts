import { describe, expect, it } from 'vitest'
import { isPaletteKey } from './paletteKey'

const k = (mods: { ctrl?: boolean; meta?: boolean; alt?: boolean } = {}, key = 'k') => ({ key, ctrlKey: !!mods.ctrl, metaKey: !!mods.meta, altKey: !!mods.alt })

describe('palette shortcut', () => {
  it('opens with Cmd+K on Mac even from a terminal, where Cmd never reaches the shell', () => {
    expect(isPaletteKey(k({ meta: true }), true, true)).toBe(true)
    expect(isPaletteKey(k({ meta: true }, 'K'), false, true)).toBe(true)
  })

  it('leaves Ctrl+K to the shell inside a terminal on every OS', () => {
    expect(isPaletteKey(k({ ctrl: true }), true, true)).toBe(false)
    expect(isPaletteKey(k({ ctrl: true }), true, false)).toBe(false)
    expect(isPaletteKey(k({ ctrl: true }), false, false)).toBe(true)
    expect(isPaletteKey(k({ ctrl: true }), false, true)).toBe(true)
  })

  it('ignores Alt, other keys and Windows Win+K in a terminal', () => {
    expect(isPaletteKey(k({ meta: true, alt: true }), false, true)).toBe(false)
    expect(isPaletteKey(k({ meta: true }, 'j'), false, true)).toBe(false)
    expect(isPaletteKey(k({ meta: true }), true, false)).toBe(false)
  })
})
