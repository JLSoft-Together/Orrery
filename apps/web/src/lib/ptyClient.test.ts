import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('./desktop', () => ({ desktop: { ptyEndpoint: async () => ({ url: 'ws://127.0.0.1:1/pty', protocol: 'ccm.pty.t' }) } }))

class FakeSocket {
  static all: FakeSocket[] = []
  static readonly OPEN = 1
  readyState = 1
  binaryType = ''
  sent: string[] = []
  onopen: (() => void) | null = null
  onmessage: ((ev: { data: unknown }) => void) | null = null
  onclose: ((ev: { code: number }) => void) | null = null
  onerror: (() => void) | null = null
  constructor(readonly url: string) {
    FakeSocket.all.push(this)
  }
  send(data: string): void {
    this.sent.push(data)
  }
  close(): void {
    this.readyState = 3
  }
}

const { PtyConnection } = await import('./ptyClient')

const ready = { t: 'ready', id: 'sess-1', preset: 'shell', pid: 1, cwd: '/', replay: false }

function setup() {
  const onOutput = vi.fn()
  const onMessage = vi.fn()
  const onClose = vi.fn()
  return { onOutput, onMessage, onClose, conn: new PtyConnection({ onOutput, onMessage, onClose }) }
}

describe('PtyConnection close 4001 (overflow)', () => {
  beforeEach(() => {
    FakeSocket.all = []
    vi.stubGlobal('WebSocket', FakeSocket)
  })
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('attaches again by id with the last size and does not report the pane closed', async () => {
    const h = setup()
    await h.conn.open({ spawn: { preset: 'shell' } }, 100, 30)
    const first = FakeSocket.all[0]!
    first.onopen?.()
    first.onmessage?.({ data: JSON.stringify(ready) })
    h.conn.resize(120, 40)
    first.onclose?.({ code: 4001 })
    await vi.waitFor(() => expect(FakeSocket.all).toHaveLength(2))
    const second = FakeSocket.all[1]!
    second.onopen?.()
    expect(JSON.parse(second.sent[0]!)).toEqual({ t: 'attach', id: 'sess-1' })
    expect(h.onClose).not.toHaveBeenCalled()
    expect(h.onOutput).toHaveBeenCalledWith(new Uint8Array([0x1b, 0x63]))
    second.onmessage?.({ data: JSON.stringify({ ...ready, replay: true }) })
    expect(h.onMessage).toHaveBeenLastCalledWith({ ...ready, replay: true })
  })

  it('reports other close codes as before', async () => {
    const h = setup()
    await h.conn.open({ spawn: { preset: 'shell' } }, 80, 24)
    const ws = FakeSocket.all[0]!
    ws.onmessage?.({ data: JSON.stringify(ready) })
    ws.onclose?.({ code: 4000 })
    expect(h.onClose).toHaveBeenCalledWith(4000)
    expect(FakeSocket.all).toHaveLength(1)
  })

  it('reports the close when the session id is not known yet', async () => {
    const h = setup()
    await h.conn.open({ spawn: { preset: 'shell' } }, 80, 24)
    FakeSocket.all[0]!.onclose?.({ code: 4001 })
    expect(h.onClose).toHaveBeenCalledWith(4001)
  })

  it('gives up after repeated overflows in a short time', async () => {
    const h = setup()
    await h.conn.open({ attach: 'sess-1' }, 80, 24)
    let ws = FakeSocket.all[0]!
    ws.onmessage?.({ data: JSON.stringify(ready) })
    for (let i = 1; i <= 5; i++) {
      ws.onclose?.({ code: 4001 })
      await vi.waitFor(() => expect(FakeSocket.all).toHaveLength(i + 1))
      ws = FakeSocket.all[i]!
    }
    ws.onclose?.({ code: 4001 })
    expect(h.onClose).toHaveBeenCalledWith(4001)
    expect(FakeSocket.all).toHaveLength(6)
  })

  it('does nothing when the owner closed the connection', async () => {
    const h = setup()
    await h.conn.open({ spawn: { preset: 'shell' } }, 80, 24)
    const ws = FakeSocket.all[0]!
    ws.onmessage?.({ data: JSON.stringify(ready) })
    h.conn.close()
    ws.onclose?.({ code: 4001 })
    expect(FakeSocket.all).toHaveLength(1)
    expect(h.onClose).not.toHaveBeenCalled()
  })
})

describe('PtyConnection claim (a phone may be viewing)', () => {
  beforeEach(() => {
    FakeSocket.all = []
    vi.stubGlobal('WebSocket', FakeSocket)
  })
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  async function open() {
    const h = setup()
    await h.conn.open({ attach: 'sess-1' }, 100, 30)
    const ws = FakeSocket.all[0]!
    ws.onopen?.()
    ws.onmessage?.({ data: JSON.stringify(ready) })
    return { ...h, ws, frames: () => ws.sent.map((s) => JSON.parse(s) as { t: string }).filter((m) => m.t === 'claim') }
  }

  it('claim sends the claim message', async () => {
    const h = await open()
    h.conn.claim()
    expect(h.frames()).toEqual([{ t: 'claim' }])
  })

  it('a lone desktop socket never claims: nothing was resized by another viewer', async () => {
    const h = await open()
    expect(h.conn.claimIfNeeded()).toBe(false)
    expect(h.frames()).toEqual([])
  })

  it('claims once after a resized notice, then stays quiet until the next one', async () => {
    const h = await open()
    h.ws.onmessage?.({ data: JSON.stringify({ t: 'resized', cols: 45, rows: 30 }) })
    expect(h.onMessage).toHaveBeenLastCalledWith({ t: 'resized', cols: 45, rows: 30 })
    expect(h.conn.claimIfNeeded()).toBe(true)
    expect(h.conn.claimIfNeeded()).toBe(false)
    expect(h.frames()).toHaveLength(1)
    h.ws.onmessage?.({ data: JSON.stringify({ t: 'resized', cols: 50, rows: 30 }) })
    expect(h.conn.claimIfNeeded()).toBe(true)
    expect(h.frames()).toHaveLength(2)
  })
})
