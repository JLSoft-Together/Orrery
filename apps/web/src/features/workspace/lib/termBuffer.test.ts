import { describe, expect, it } from 'vitest'
import { HiddenOutput } from './termBuffer'

const enc = new TextEncoder()
const dec = new TextDecoder()

function setup(max?: number) {
  const writes: Uint8Array[] = []
  const timers: { fn: () => void; cancelled: boolean }[] = []
  const out = new HiddenOutput({
    write: (bytes, done) => {
      writes.push(bytes.slice())
      done?.()
    },
    max,
    later: (fn) => {
      const t = { fn, cancelled: false }
      timers.push(t)
      return () => (t.cancelled = true)
    },
  })
  const runTimers = (): void => {
    for (const t of timers.splice(0)) if (!t.cancelled) t.fn()
  }
  const live = (): number => timers.filter((t) => !t.cancelled).length
  const text = (): string => writes.map((w) => dec.decode(w)).join('')
  return { out, writes, runTimers, live, text }
}

describe('HiddenOutput', () => {
  it('writes straight through while visible', () => {
    const { out, writes } = setup()
    out.push(enc.encode('a'))
    out.push(enc.encode('b'))
    expect(writes.map((w) => dec.decode(w))).toEqual(['a', 'b'])
  })

  it('holds chunks while hidden and writes them once, in order, when shown', () => {
    const { out, writes, text } = setup()
    out.setVisible(false)
    const parts = ['hello ', '\x1b[3', '1mred', '\x1b[0m', ' wörld', '\r\n']
    for (const p of parts) out.push(enc.encode(p))
    expect(writes).toHaveLength(0)
    expect(out.pending).toBe(parts.reduce((n, p) => n + enc.encode(p).length, 0))
    let done = false
    out.setVisible(true, () => (done = true))
    expect(done).toBe(true)
    expect(writes).toHaveLength(1)
    expect(text()).toBe(parts.join(''))
    expect(out.pending).toBe(0)
  })

  it('keeps byte order across hidden, flushed and live writes', () => {
    const { out, text, runTimers } = setup()
    out.setVisible(false)
    out.push(enc.encode('1'))
    out.push(enc.encode('2'))
    runTimers()
    out.push(enc.encode('3'))
    out.setVisible(true)
    out.push(enc.encode('4'))
    expect(text()).toBe('1234')
  })

  it('flushes in a batch on the idle timer, one timer per batch', () => {
    const { out, writes, runTimers, live } = setup()
    out.setVisible(false)
    out.push(enc.encode('a'))
    out.push(enc.encode('b'))
    expect(live()).toBe(1)
    runTimers()
    expect(writes.map((w) => dec.decode(w))).toEqual(['ab'])
    expect(live()).toBe(0)
  })

  it('writes at once when the program asks the terminal for a reply', () => {
    for (const q of ['\x1b[6n', '\x1b[c', '\x1b[>c', '\x1b[?2026$p', '\x1b]11;?\x07', '\x1b[?u', '\x1b[14t', '\x1bP$qm\x1b\\']) {
      const { out, writes } = setup()
      out.setVisible(false)
      out.push(enc.encode('x'))
      out.push(enc.encode(q))
      expect(writes.map((w) => dec.decode(w)), JSON.stringify(q)).toEqual(['x' + q])
    }
  })

  it('spots a reply request split across chunks', () => {
    const { out, writes } = setup()
    out.setVisible(false)
    out.push(enc.encode('ab\x1b['))
    expect(writes).toHaveLength(0)
    out.push(enc.encode('6n'))
    expect(writes.map((w) => dec.decode(w))).toEqual(['ab\x1b[6n'])
  })

  it('does not flush on ordinary styling', () => {
    const { out, writes } = setup()
    out.setVisible(false)
    out.push(enc.encode('\x1b[1;31mred\x1b[0m\x1b[2K\x1b[?25l\x1b]0;title\x07'))
    expect(writes).toHaveLength(0)
  })

  it('writes everything at once past max, never dropping bytes', () => {
    const { out, writes, text } = setup(20)
    out.setVisible(false)
    out.push(enc.encode('\x1b[?1049h'))
    out.push(enc.encode('0123'))
    expect(writes).toHaveLength(0)
    out.push(enc.encode('456789abcd'))
    expect(writes).toHaveLength(1)
    expect(out.pending).toBe(0)
    expect(text()).toBe('\x1b[?1049h0123456789abcd')
  })

  it('spots a reply request split across an idle flush', () => {
    const { out, writes, runTimers } = setup()
    out.setVisible(false)
    out.push(enc.encode('ab\x1b[6'))
    runTimers()
    expect(writes).toHaveLength(1)
    out.push(enc.encode('n'))
    expect(writes.map((w) => dec.decode(w))).toEqual(['ab\x1b[6', 'n'])
  })

  it('spots a reply request split across the switch to hidden', () => {
    const { out, writes } = setup()
    out.push(enc.encode('x'.repeat(100) + '\x1b[6'))
    out.setVisible(false)
    out.push(enc.encode('n'))
    expect(writes).toHaveLength(2)
    expect(dec.decode(writes[1]!)).toBe('n')
  })

  it('does not flush again for a request already answered', () => {
    const { out, writes } = setup()
    out.push(enc.encode('\x1b[6n'))
    out.setVisible(false)
    out.push(enc.encode('a'))
    out.push(enc.encode('b'))
    expect(writes).toHaveLength(1)
    expect(out.pending).toBe(2)
  })

  it('drops everything and cancels the timer on dispose', () => {
    const { out, writes, runTimers, live } = setup()
    out.setVisible(false)
    out.push(enc.encode('late'))
    out.dispose()
    expect(live()).toBe(0)
    runTimers()
    expect(writes).toHaveLength(0)
    expect(out.pending).toBe(0)
  })

  it('clear forgets bytes from an old stream', () => {
    const { out, text } = setup()
    out.setVisible(false)
    out.push(enc.encode('old'))
    out.clear()
    out.push(enc.encode('new'))
    out.setVisible(true)
    expect(text()).toBe('new')
  })
})
