import { describe, expect, it } from 'vitest'
import { dropLastChar, droppedInsert, replacementBytes, textareaSyncFor } from './termIme'

const key = (k: string, extra: Partial<KeyboardEvent> = {}) => ({ key: k, keyCode: 0, isComposing: false, ctrlKey: false, metaKey: false, altKey: false, ...extra })

describe('replacementBytes', () => {
  it('sends only the changed tail of a Telex word', () => {
    expect(replacementBytes('v', 0, 1, 'vi')).toBe('i')
    expect(replacementBytes('vie', 0, 3, 'viê')).toBe('\x7fê')
    expect(replacementBytes('viêt', 0, 4, 'việt')).toBe('\x7f\x7fệt')
  })

  it('only looks at the replaced range', () => {
    expect(replacementBytes('ls tieng', 3, 8, 'tiếng')).toBe('\x7f\x7f\x7fếng')
  })

  it('counts code points, not UTF-16 units', () => {
    expect(replacementBytes('a😀', 0, 2, 'b')).toBe('\x7f\x7fb')
  })

  it('handles empty and out-of-range selections', () => {
    expect(replacementBytes('', 0, 0, 'ơ')).toBe('ơ')
    expect(replacementBytes('abc', 2, 1, 'x')).toBe('x')
    expect(replacementBytes('abc', 0, 3, 'abc')).toBe('')
  })
})

describe('textareaSyncFor', () => {
  it('trims on a plain Backspace', () => {
    expect(textareaSyncFor(key('Backspace'))).toBe('trim')
    expect(textareaSyncFor(key('Backspace', { altKey: true }))).toBe('clear')
  })

  it('clears on Enter, arrows and shortcuts', () => {
    expect(textareaSyncFor(key('Enter'))).toBe('clear')
    expect(textareaSyncFor(key('ArrowLeft'))).toBe('clear')
    expect(textareaSyncFor(key('v', { metaKey: true }))).toBe('clear')
    expect(textareaSyncFor(key('c', { ctrlKey: true }))).toBe('clear')
  })

  it('keeps for printable keys, modifiers and IME keys', () => {
    expect(textareaSyncFor(key('a'))).toBe('keep')
    expect(textareaSyncFor(key('Shift'))).toBe('keep')
    expect(textareaSyncFor(key('e', { keyCode: 229 }))).toBe('keep')
    expect(textareaSyncFor(key('Enter', { isComposing: true }))).toBe('keep')
  })
})

describe('droppedInsert', () => {
  const base = { inputType: 'insertText', data: 't', composing: false, inComposition: false, sentByTerminal: false, pressedKey: null }

  it('resends a letter the terminal ignored after a space keypress', () => {
    expect(droppedInsert({ ...base, pressedKey: ' ' })).toBe('t')
  })

  it('skips letters the terminal already sent', () => {
    expect(droppedInsert({ ...base, sentByTerminal: true })).toBeNull()
    expect(droppedInsert({ ...base, pressedKey: 't' })).toBeNull()
  })

  it('ignores composition, replacements and empty data', () => {
    expect(droppedInsert({ ...base, composing: true })).toBeNull()
    expect(droppedInsert({ ...base, inComposition: true })).toBeNull()
    expect(droppedInsert({ ...base, inputType: 'insertReplacementText' })).toBeNull()
    expect(droppedInsert({ ...base, data: null })).toBeNull()
  })
})

describe('dropLastChar', () => {
  it('removes one code point', () => {
    expect(dropLastChar('việt')).toBe('việ')
    expect(dropLastChar('a😀')).toBe('a')
    expect(dropLastChar('')).toBe('')
  })
})
