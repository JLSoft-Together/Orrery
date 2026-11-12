import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { RemoteCommandError, type JsonMap } from '../remote/types'
import { keySequence, lineBytes, utf8 } from './keys'
import { controlChar, StickyModifiers } from './modifiers'
import { PtyStreams, type PtyTransport } from './ptyStreams'

describe('sticky modifiers', () => {
  it('cycles off -> once -> locked -> off', () => {
    const m = new StickyModifiers()
    m.toggleCtrl()
    expect(m.ctrl).toBe('once')
    m.toggleCtrl()
    expect(m.ctrl).toBe('locked')
    m.toggleCtrl()
    expect(m.ctrl).toBe('off')
  })

  it('once is consumed by the next character, locked stays', () => {
    const m = new StickyModifiers()
    m.toggleCtrl()
    expect(m.apply('c')).toBe('\x03')
    expect(m.ctrl).toBe('off')
    expect(m.apply('c')).toBe('c')
    m.toggleCtrl()
    m.toggleCtrl()
    expect(m.apply('a')).toBe('\x01')
    expect(m.apply('z')).toBe('\x1a')
    expect(m.ctrl).toBe('locked')
  })

  it('alt prefixes escape and combines with ctrl', () => {
    const m = new StickyModifiers()
    m.toggleAlt()
    expect(m.apply('b')).toBe('\x1bb')
    m.toggleCtrl()
    expect(m.apply('d')).toBe('\x04')
    m.toggleAlt()
    m.toggleCtrl()
    expect(m.apply('d')).toBe('\x1b\x04')
  })

  it('leaves control sequences and multi-character data alone', () => {
    const m = new StickyModifiers()
    m.toggleCtrl()
    expect(m.apply('\r')).toBe('\r')
    expect(m.apply('\x1b[A')).toBe('\x1b[A')
    expect(m.apply('xin chào')).toBe('xin chào')
    expect(m.ctrl).toBe('once')
  })

  it('control characters follow the ASCII map', () => {
    expect(controlChar(' ')).toBe('\x00')
    expect(controlChar('?')).toBe('\x7f')
    expect(controlChar('[')).toBe('\x1b')
    expect(controlChar('1')).toBe('1')
  })
})

describe('key bytes', () => {
  it('maps the extra keys', () => {
    expect(keySequence({ key: 'esc', ctrl: false, alt: false })).toBe('\x1b')
    expect(keySequence({ key: 'tab', ctrl: false, alt: false })).toBe('\t')
    expect(keySequence({ key: 'shiftTab', ctrl: false, alt: false })).toBe('\x1b[Z')
    expect(keySequence({ key: 'ctrlC', ctrl: false, alt: false })).toBe('\x03')
    expect(keySequence({ key: 'enter', ctrl: false, alt: false })).toBe('\r')
    expect(keySequence({ key: 'up', ctrl: false, alt: false })).toBe('\x1b[A')
    expect(keySequence({ key: 'left', ctrl: false, alt: false })).toBe('\x1b[D')
  })

  it('modified arrows use the xterm modifier parameter', () => {
    expect(keySequence({ key: 'right', ctrl: true, alt: false })).toBe('\x1b[1;5C')
    expect(keySequence({ key: 'left', ctrl: false, alt: true })).toBe('\x1b[1;3D')
    expect(keySequence({ key: 'up', ctrl: true, alt: true })).toBe('\x1b[1;7A')
  })

  it('alt prefixes escape on other keys', () => {
    expect(keySequence({ key: 'tab', ctrl: false, alt: true })).toBe('\x1b\t')
    expect(keySequence({ key: 'enter', ctrl: false, alt: true })).toBe('\x1b\r')
  })

  it('a typed line ends with Enter and is bracketed only when the app asked for it', () => {
    expect(lineBytes('echo xin chào', false)).toBe('echo xin chào\r')
    expect(lineBytes('echo xin chào', true)).toBe('\x1b[200~echo xin chào\x1b[201~\r')
    expect(lineBytes('a\nb', false)).toBe('a\rb\r')
  })

  it('encodes Vietnamese as UTF-8', () => {
    expect([...utf8('chào')]).toEqual([0x63, 0x68, 0xc3, 0xa0, 0x6f])
  })
})

interface Sent {
  stream: number
  bytes: number[]
  claim: boolean
}

function transport(): { t: PtyTransport; sent: Sent[]; requests: Array<{ type: string; fields: JsonMap }>; state: { ready: boolean; fail: string | null } } {
  const sent: Sent[] = []
  const requests: Array<{ type: string; fields: JsonMap }> = []
  const state = { ready: true, fail: null as string | null }
  return {
    sent,
    requests,
    state,
    t: {
      ready: () => state.ready,
      request: async (type, fields) => {
        requests.push({ type, fields })
        if (state.fail) throw new RemoteCommandError(state.fail)
        return { ok: true }
      },
      sendInput: (stream, bytes, claim) => sent.push({ stream, bytes: [...bytes], claim }),
    },
  }
}

describe('PtyStreams', () => {
  beforeEach(() => vi.useFakeTimers())
  afterEach(() => vi.useRealTimers())

  const handlers = { onOutput: () => {}, onEvent: () => {} }

  it('allocates ids, attaches with the size and limits to 4', async () => {
    const x = transport()
    const s = new PtyStreams(x.t)
    const a = await s.attach('sess', 80, 24, handlers)
    expect(a.stream).toBe(1)
    expect(x.requests[0]).toEqual({ type: 'pty.attach', fields: { stream: 1, sessionId: 'sess', cols: 80, rows: 24 } })
    await s.attach('b', 80, 24, handlers)
    await s.attach('c', 80, 24, handlers)
    await s.attach('d', 80, 24, handlers)
    await expect(s.attach('e', 80, 24, handlers)).rejects.toMatchObject({ code: 'limit' })
    expect(s.count).toBe(4)
  })

  it('a failed attach frees its slot', async () => {
    const x = transport()
    x.state.fail = 'notFound'
    const s = new PtyStreams(x.t)
    await expect(s.attach('gone', 80, 24, handlers)).rejects.toMatchObject({ code: 'notFound' })
    expect(s.count).toBe(0)
  })

  it('sends the first input at once with its claim and merges the rest in 30 ms, claim ORed', async () => {
    const x = transport()
    const s = new PtyStreams(x.t)
    const a = await s.attach('sess', 80, 24, handlers)
    s.input(a, new Uint8Array([1]), true)
    s.input(a, new Uint8Array([2]), false)
    s.input(a, new Uint8Array([3, 4]), false)
    expect(x.sent).toEqual([{ stream: 1, bytes: [1], claim: true }])
    vi.advanceTimersByTime(31)
    expect(x.sent[1]).toEqual({ stream: 1, bytes: [2, 3, 4], claim: false })
    s.input(a, new Uint8Array([5]), false)
    s.input(a, new Uint8Array([6]), true)
    vi.advanceTimersByTime(31)
    expect(x.sent[2]).toEqual({ stream: 1, bytes: [5, 6], claim: true })
  })

  it('does not send while suspended or offline', async () => {
    const x = transport()
    const s = new PtyStreams(x.t)
    const a = await s.attach('sess', 80, 24, handlers)
    x.state.ready = false
    s.input(a, new Uint8Array([1]), true)
    expect(x.sent).toEqual([])
    x.state.ready = true
    await s.suspendAll()
    s.input(a, new Uint8Array([1]), true)
    expect(x.sent).toEqual([])
    expect(x.requests.at(-1)?.type).toBe('pty.detach')
    await s.resumeAll()
    expect(x.requests.at(-1)?.type).toBe('pty.attach')
    s.input(a, new Uint8Array([9]), true)
    expect(x.sent).toEqual([{ stream: 1, bytes: [9], claim: true }])
  })

  it('routes output and events by stream and re-attaches after pty.overflow', async () => {
    const x = transport()
    const s = new PtyStreams(x.t)
    const got: number[][] = []
    const events: string[] = []
    const a = await s.attach('sess', 80, 24, { onOutput: (b) => got.push([...b]), onEvent: (e) => events.push(String(e.t)) })
    s.feed(a.stream, new Uint8Array([7, 8]))
    s.feed(99, new Uint8Array([1]))
    expect(got).toEqual([[7, 8]])
    s.event({ t: 'pty.ready', stream: 1, cols: 100, rows: 30 })
    s.event({ t: 'pty.ready', stream: 2 })
    s.event({ t: 'pty.overflow', stream: 1 })
    await vi.advanceTimersByTimeAsync(0)
    expect(events).toEqual(['pty.ready', 'pty.overflow'])
    expect(x.requests.map((r) => r.type)).toEqual(['pty.attach', 'pty.detach', 'pty.attach'])
  })

  it('re-attaches every live stream when the link comes back', async () => {
    const x = transport()
    const s = new PtyStreams(x.t)
    await s.attach('one', 80, 24, handlers)
    await s.attach('two', 90, 30, handlers)
    x.requests.length = 0
    await s.onReady()
    expect(x.requests.map((r) => r.fields.sessionId)).toEqual(['one', 'two'])
  })

  it('detach tells the desktop once and ignores a second call', async () => {
    const x = transport()
    const s = new PtyStreams(x.t)
    const a = await s.attach('sess', 80, 24, handlers)
    await s.detach(a)
    await s.detach(a)
    expect(x.requests.filter((r) => r.type === 'pty.detach')).toHaveLength(1)
    expect(s.count).toBe(0)
  })

  it('resize remembers the size and only asks the desktop when ready', async () => {
    const x = transport()
    const s = new PtyStreams(x.t)
    const a = await s.attach('sess', 80, 24, handlers)
    await s.resize(a, 50, 20)
    expect(x.requests.at(-1)).toEqual({ type: 'pty.resize', fields: { stream: 1, cols: 50, rows: 20 } })
    x.state.ready = false
    await s.resize(a, 40, 10)
    expect([a.cols, a.rows]).toEqual([40, 10])
    expect(x.requests.filter((r) => r.type === 'pty.resize')).toHaveLength(1)
  })
})
