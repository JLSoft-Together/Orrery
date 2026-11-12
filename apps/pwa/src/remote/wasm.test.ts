import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { REMOTE_PROTO } from '@ccm/shared'
import { beforeAll, describe, expect, it } from 'vitest'
import { DEFAULT_CONFIG } from './types'
import { NoiseError, useWasmBytes, type NoiseSuite } from './noise'

let noise: NoiseSuite

beforeAll(() => {
  noise = useWasmBytes(readFileSync(join(import.meta.dirname, '../gen/core_remote_wasm_bg.wasm')))
})

describe('wasm noise suite', () => {
  it('matches the shared protocol version', () => {
    expect(noise.protoVersion()).toBe(REMOTE_PROTO)
    expect(DEFAULT_CONFIG.proto).toBe(REMOTE_PROTO)
  })

  it('generates distinct 32 byte private keys', () => {
    const a = noise.generatePrivateKey()
    const b = noise.generatePrivateKey()
    expect(a).toHaveLength(32)
    expect([...a]).not.toEqual([...b])
  })

  it('rejects a malformed QR with a stable error code', () => {
    expect(() => noise.decodeQr('not a qr')).toThrow(NoiseError)
    try {
      noise.decodeQr('%%%')
    } catch (error) {
      expect((error as NoiseError).code).toBe('badQr')
    }
  })

  it('starts an IK session of the right size and rejects a corrupted msg2', () => {
    const key = noise.generatePrivateKey()
    const desktop = new Uint8Array(32).fill(9)
    const start = noise.sessionStart(key, desktop, 'pwa-test')
    expect(start.msg1.length).toBeGreaterThan(50)
    expect(() => start.finish(new Uint8Array(48))).toThrow(NoiseError)
    start.dispose()
  })

  it('rejects a bad desktop key length', () => {
    expect(() => noise.sessionStart(noise.generatePrivateKey(), new Uint8Array(5), 'x')).toThrow(NoiseError)
  })
})
