import { describe, expect, it } from 'vitest'
import { createPairFlow, PairError, pairCandidates } from './pairing'
import type { NoiseSuite, QrView } from './noise'
import { toHex } from './bytes'
import { connectionDeps, createNetwork, FakeLink, jsonFrame, MemoryMachines, type FakeSocket } from '../test/fakes'
import { fakeNoise } from '../test/fakes'

const mkey = new Uint8Array(32).fill(5)
const view: QrView = { v: 1, machine: 'm9', name: 'Desk', host: null, port: null, relay: 'https://relay.example', mkey: toHex(mkey), exp: 0 }

function suite(): NoiseSuite {
  return {
    ...fakeNoise,
    decodeQr: () => view,
    pairFromQr: () => ({
      msg1: new Uint8Array([10]),
      finish: (msg2) => {
        if (msg2[0] !== 20) throw new Error('keyMismatch')
        return new Uint8Array([30])
      },
      desktopStatic: () => mkey,
      intoLink: () => new FakeLink(),
      dispose: () => {},
    }),
  }
}

function flow(reply: unknown) {
  const net = createNetwork()
  const sockets: FakeSocket[] = []
  net.routes.set('wss://relay.example/p/m9?t=boot', (socket) => {
    sockets.push(socket)
    socket.onClientSend = (data) => {
      if (data[0] === 10) socket.serverPush(new Uint8Array([20]))
      if (data[0] === 30) socket.serverPush(jsonFrame(reply))
    }
  })
  return { flow: createPairFlow({ ...connectionDeps(net, new MemoryMachines()), noise: suite() }), sockets }
}

describe('pair flow', () => {
  it('pairs over the relay with a bootstrap token and returns the machine', async () => {
    const { flow: f, sockets } = flow({ t: 'pair.ok', machine: 'm9', name: 'Real name', relay: 'https://relay.example', relayToken: 'x' })
    const machine = await f.pair({ qr: 'qr', token: 'boot', deviceName: 'phone' })
    expect(machine).toMatchObject({ id: 'm9', name: 'Real name', relayUrl: 'https://relay.example', policy: 'auto' })
    expect([...machine.mkey]).toEqual([...mkey])
    expect(sockets[0]?.isClosed).toBe(true)
  })

  it('maps pair.denied to a PairError', async () => {
    const { flow: f } = flow({ t: 'pair.denied' })
    await expect(f.pair({ qr: 'qr', token: 'boot', deviceName: 'phone' })).rejects.toMatchObject({ code: 'denied' })
  })

  it('has no route without a token or LAN host', async () => {
    const { flow: f } = flow({})
    await expect(f.pair({ qr: 'qr', deviceName: 'phone' })).rejects.toBeInstanceOf(PairError)
  })
})

describe('pairCandidates', () => {
  const base: QrView = { ...view, lan: [{ host: '192.168.1.5', port: 4320 }, { host: '10.0.0.4', port: 4321 }], pt: 'ticket 1' }

  it('tries every LAN address first, then the relay with the ticket', () => {
    const list = pairCandidates(base, {}, 3000, 8000, 4320)
    expect(list.map((c) => c.url)).toEqual(['wss://192.168.1.5:4320/remote', 'wss://10.0.0.4:4321/remote', 'wss://relay.example/p/m9?pt=ticket%201'])
    expect(list.map((c) => c.budgetMs)).toEqual([3000, 3000, 8000])
  })

  it('falls back to a v1 host and a bootstrap token', () => {
    const v1: QrView = { ...view, host: '10.1.1.1', port: null, lan: undefined, pt: undefined }
    expect(pairCandidates(v1, { token: 'abc' }, 1, 2, 4320).map((c) => c.url)).toEqual(['wss://10.1.1.1:4320/remote', 'wss://relay.example/p/m9?t=abc'])
  })

  it('has nothing to dial without any route', () => {
    expect(pairCandidates({ ...view, relay: null }, {}, 1, 2, 4320)).toEqual([])
  })
})
