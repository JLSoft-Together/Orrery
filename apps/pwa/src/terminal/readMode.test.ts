import { describe, expect, it } from 'vitest'
import { Terminal } from '@xterm/xterm'
import { bufferParagraphs, loadMode, nextMode, readerText, saveMode } from './readMode'

function fake(rows: Array<[string, boolean]>) {
  return {
    length: rows.length,
    getLine: (y: number) => (rows[y] ? { translateToString: (trim?: boolean) => (trim ? rows[y]![0].trimEnd() : rows[y]![0]), isWrapped: rows[y]![1] } : undefined),
  }
}

function mem() {
  const m = new Map<string, string>()
  return { getItem: (k: string) => m.get(k) ?? null, setItem: (k: string, v: string) => void m.set(k, v), removeItem: (k: string) => void m.delete(k), m }
}

describe('reader text', () => {
  it('joins wrapped rows and trims trailing blank rows', () => {
    const b = fake([
      ['hello wor', false],
      ['ld again', true],
      ['next', false],
      ['   ', false],
      ['', false],
    ])
    expect(bufferParagraphs(b)).toEqual(['hello world again', 'next', '', ''])
    expect(readerText(b)).toBe('hello world again\nnext')
  })

  it('keeps the space at a wrap boundary', () => {
    expect(readerText(fake([['abc ', false], ['def', true]]))).toBe('abc def')
  })

  it('reads a real xterm buffer written by a snapshot', async () => {
    const t = new Terminal({ cols: 10, rows: 4, allowProposedApi: true })
    await new Promise<void>((r) => t.write('0123456789abcdef\r\nok', r))
    expect(readerText(t.buffer.active)).toBe('0123456789abcdef\nok')
  })
})

describe('terminal mode', () => {
  it('toggle flips, input always returns to desk', () => {
    expect(nextMode('desk', 'toggle')).toBe('read')
    expect(nextMode('read', 'toggle')).toBe('desk')
    expect(nextMode('read', 'input')).toBe('desk')
    expect(nextMode('desk', 'input')).toBe('desk')
  })

  it('remembers read per session and defaults to desk', () => {
    const s = mem()
    expect(loadMode('a', s)).toBe('desk')
    saveMode('a', 'read', s)
    expect(loadMode('a', s)).toBe('read')
    expect(loadMode('b', s)).toBe('desk')
    saveMode('a', 'desk', s)
    expect(loadMode('a', s)).toBe('desk')
    expect(s.m.size).toBe(0)
  })

  it('survives a storage that throws', () => {
    const bad = {
      getItem: () => {
        throw new Error('x')
      },
      setItem: () => {
        throw new Error('x')
      },
      removeItem: () => {
        throw new Error('x')
      },
    }
    expect(loadMode('a', bad)).toBe('desk')
    expect(() => saveMode('a', 'read', bad)).not.toThrow()
    expect(loadMode(null, bad)).toBe('desk')
  })
})

describe('ctrl row keys', () => {
  it('send Ctrl+D, Ctrl+L, Ctrl+R control bytes', async () => {
    const { keySequence } = await import('./keys')
    expect(keySequence({ key: 'ctrlD', ctrl: false, alt: false })).toBe('\x04')
    expect(keySequence({ key: 'ctrlL', ctrl: false, alt: true })).toBe('\x0c')
    expect(keySequence({ key: 'ctrlR', ctrl: false, alt: false })).toBe('\x12')
    expect(keySequence({ key: 'ctrlC', ctrl: false, alt: false })).toBe('\x03')
  })
})
